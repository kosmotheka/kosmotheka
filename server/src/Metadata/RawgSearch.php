<?php

declare(strict_types=1);

namespace App\Metadata;

use App\Entity\MetadataCacheEntry;
use App\Repository\MetadataCacheRepository;
use App\Security\AccountException;
use App\Security\RequestLimiter;
use Doctrine\DBAL\Exception\UniqueConstraintViolationException;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;

/**
 * Searches video games by title for an account (7.8, D62): the cache first, then RAWG, within what the server allows
 * itself. What is kept is the result of a search under a fingerprint of what was searched, never the text, and each
 * detail under RAWG's own number; nothing says who asked. A failure is never kept.
 */
final readonly class RawgSearch
{
    /** Only the first results get a second call: a search costs up to four. */
    private const int DETAILS_OF = 3;
    private const string DETAIL_PROVIDER = 'rawg_game';

    public function __construct(
        private RawgSource $source,
        private MetadataCacheRepository $cache,
        private EntityManagerInterface $entityManager,
        private RequestLimiter $limiter,
        private ClockInterface $clock,
        private ProviderGate $gate,
    ) {
    }

    public static function fingerprint(string $query): string
    {
        return hash('sha256', "rawg\n".self::normalise($query));
    }

    /** Trimmed, with every run of blanks (non-breaking ones too) as one space, and lower case. */
    public static function normalise(string $query): string
    {
        return mb_strtolower(trim((string) preg_replace('/[\s\x{a0}]+/u', ' ', $query)));
    }

    /**
     * @return list<array{external_id: int, title: ?string, year: ?int, cover_url: ?string, platforms: list<string>, developers: list<string>, publishers: list<string>}>
     *
     * @throws AccountException with AccountErrorCode::MetadataQuotaExhausted or MetadataFailed
     */
    public function search(string $query): array
    {
        $now = $this->clock->now();
        $hits = $this->hits(self::normalise($query), $now);
        $details = $this->details(\array_slice($hits, 0, self::DETAILS_OF), $now);

        $candidates = [];
        foreach ($hits as $hit) {
            $detail = $details[$hit->id] ?? new RawgDetail();
            $candidates[] = $hit->toArray() + $detail->toArray();
        }

        return $candidates;
    }

    /**
     * @return list<RawgHit>
     *
     * @throws AccountException
     */
    private function hits(string $query, \DateTimeImmutable $now): array
    {
        $code = self::fingerprint($query);
        $entry = $this->cache->findOneByProviderAndCode(MetadataCacheEntry::RAWG_SEARCH, $code);
        if ($entry instanceof MetadataCacheEntry && $entry->isFreshAt($now)) {
            $stored = $entry->getPayload()['hits'] ?? [];

            return $this->hitsFrom(\is_array($stored) ? array_values($stored) : []);
        }

        $this->gate->requireOpen(RawgSource::NAME);
        $hits = $this->gate->call(
            RawgSource::NAME,
            $this->limiter->consumeRawgCall(...),
            fn (): array => $this->source->search($query),
        );

        $this->remember($entry, MetadataCacheEntry::RAWG_SEARCH, $code, [] !== $hits, ['hits' => array_map(static fn (RawgHit $hit): array => $hit->toArray(), $hits)], $now);

        return $hits;
    }

    /**
     * @param list<RawgHit> $hits
     *
     * @return array<int, RawgDetail>
     */
    private function details(array $hits, \DateTimeImmutable $now): array
    {
        $details = [];
        $missing = [];
        $entries = [];
        foreach ($hits as $hit) {
            $entry = $this->cache->findOneByProviderAndCode(self::DETAIL_PROVIDER, (string) $hit->id);
            if ($entry instanceof MetadataCacheEntry && $entry->isFreshAt($now)) {
                $details[$hit->id] = RawgDetail::fromArray($entry->getPayload());
                continue;
            }
            $entries[$hit->id] = $entry;
            $missing[] = $hit->id;
        }
        if ([] === $missing) {
            return $details;
        }

        try {
            $this->gate->requireOpen(RawgSource::NAME);
        } catch (AccountException) {
            // RAWG asked us to leave it alone: the search stands without its details.
            return $details;
        }

        $fetched = $this->source->details(
            $missing,
            function (): bool {
                try {
                    $this->limiter->consumeRawgCall();
                } catch (AccountException) {
                    return false;
                }

                return true;
            },
            fn () => $this->gate->pause(RawgSource::NAME),
        );
        foreach ($fetched as $id => $detail) {
            $details[$id] = $detail;
            $this->remember($entries[$id] ?? null, self::DETAIL_PROVIDER, (string) $id, true, $detail->toArray(), $now, false);
        }
        $this->flush();

        return $details;
    }

    /**
     * @param list<mixed> $stored
     *
     * @return list<RawgHit>
     */
    private function hitsFrom(array $stored): array
    {
        $hits = [];
        foreach ($stored as $data) {
            $hit = \is_array($data) ? RawgHit::fromArray($data) : null;
            if ($hit instanceof RawgHit) {
                $hits[] = $hit;
            }
        }

        return $hits;
    }

    /** @param array<string, mixed> $payload */
    private function remember(?MetadataCacheEntry $entry, string $provider, string $code, bool $found, array $payload, \DateTimeImmutable $now, bool $flush = true): void
    {
        if ($entry instanceof MetadataCacheEntry) {
            $entry->renew($found, $payload, $now);
        } else {
            $this->entityManager->persist(new MetadataCacheEntry($provider, $code, $found, $payload, $now));
        }
        if ($flush) {
            $this->flush();
        }
    }

    private function flush(): void
    {
        try {
            $this->entityManager->flush();
        } catch (UniqueConstraintViolationException) {
            // Another request stored the same a moment ago: its row is as good as ours.
        }
    }
}
