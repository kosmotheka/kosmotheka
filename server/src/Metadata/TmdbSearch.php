<?php

declare(strict_types=1);

namespace App\Metadata;

use App\Entity\MetadataCacheEntry;
use App\Enum\MediaKind;
use App\Repository\MetadataCacheRepository;
use App\Security\AccountException;
use App\Security\RequestLimiter;
use Doctrine\DBAL\Exception\UniqueConstraintViolationException;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;

/**
 * Searches films and series by title for an account (7.8, D62): the cache first, then TMDB, within what the server
 * allows itself. What is kept is the result of a search under a fingerprint of what was searched, never the text, and
 * each detail under TMDB's own number; nothing says who asked. A failure is never kept.
 */
final readonly class TmdbSearch
{
    public function __construct(
        private TmdbSource $source,
        private MetadataCacheRepository $cache,
        private EntityManagerInterface $entityManager,
        private RequestLimiter $limiter,
        private ClockInterface $clock,
        private ProviderGate $gate,
    ) {
    }

    /**
     * The key a search is kept under: a hash of the kind, the language and the text as it is asked, so the table does
     * not hold what people look for, and the same search written with other capitals or spaces is the same one.
     */
    public static function fingerprint(string $kind, string $language, string $query): string
    {
        return hash('sha256', $kind."\n".$language."\n".self::normalise($query));
    }

    /** Trimmed, with every run of blanks (non-breaking ones too) as one space, and lower case. */
    public static function normalise(string $query): string
    {
        return TextCleaner::normaliseQuery($query);
    }

    /**
     * @return list<array{external_id: int, title: ?string, original_title: ?string, year: ?int, language: ?string, cover_url: ?string, directors: list<string>, creators: list<string>, duration_minutes: ?int, seasons: ?int}>
     *
     * @throws AccountException with AccountErrorCode::MetadataQuotaExhausted or MetadataFailed
     */
    public function search(MediaKind $kind, string $language, string $query): array
    {
        $now = $this->clock->now();
        $hits = $this->hits($kind, $language, self::normalise($query), $now);
        $details = $this->details($kind, $language, $hits, $now);

        $candidates = [];
        foreach ($hits as $hit) {
            $detail = $details[$hit->id] ?? new TmdbDetail();
            $candidates[] = $hit->toArray() + $detail->toArray();
        }

        return $candidates;
    }

    /**
     * @return list<TmdbHit>
     *
     * @throws AccountException
     */
    private function hits(MediaKind $kind, string $language, string $query, \DateTimeImmutable $now): array
    {
        $code = self::fingerprint($kind->value, $language, $query);
        $entry = $this->cache->findOneByProviderAndCode(MetadataCacheEntry::TMDB_SEARCH, $code);
        if ($entry instanceof MetadataCacheEntry && $entry->isFreshAt($now)) {
            $stored = $entry->getPayload()['hits'] ?? [];

            return $this->hitsFrom(\is_array($stored) ? array_values($stored) : []);
        }

        $this->gate->requireOpen(TmdbSource::NAME);
        $hits = $this->gate->call(
            TmdbSource::NAME,
            $this->limiter->consumeTmdbCall(...),
            fn (): array => $this->source->search($kind, $language, $query),
        );

        $this->remember($entry, MetadataCacheEntry::TMDB_SEARCH, $code, [] !== $hits, ['hits' => array_map(static fn (TmdbHit $hit): array => $hit->toArray(), $hits)], $now);

        return $hits;
    }

    /**
     * The details of these hits: those kept, and asked for together those that are not. A hit whose detail cannot be
     * had is left out of the answer, not an error.
     *
     * @param list<TmdbHit> $hits
     *
     * @return array<int, TmdbDetail>
     */
    private function details(MediaKind $kind, string $language, array $hits, \DateTimeImmutable $now): array
    {
        $provider = self::detailProvider($kind, $language);
        $details = [];
        $missing = [];
        $entries = [];
        foreach ($hits as $hit) {
            $entry = $this->cache->findOneByProviderAndCode($provider, (string) $hit->id);
            if ($entry instanceof MetadataCacheEntry && $entry->isFreshAt($now)) {
                $details[$hit->id] = TmdbDetail::fromArray($entry->getPayload());
                continue;
            }
            $entries[$hit->id] = $entry;
            $missing[] = $hit->id;
        }
        if ([] === $missing) {
            return $details;
        }

        try {
            $this->gate->requireOpen(TmdbSource::NAME);
        } catch (AccountException) {
            // TMDB asked us to leave it alone: the search stands without its details.
            return $details;
        }

        $fetched = $this->source->details(
            $kind,
            $language,
            $missing,
            function (): bool {
                try {
                    $this->limiter->consumeTmdbCall();
                } catch (AccountException) {
                    return false;
                }

                return true;
            },
            fn () => $this->gate->pause(TmdbSource::NAME),
        );
        foreach ($fetched as $id => $detail) {
            $details[$id] = $detail;
            $this->remember($entries[$id] ?? null, $provider, (string) $id, true, $detail->toArray(), $now, false);
        }
        $this->flush();

        return $details;
    }

    public static function detailProvider(MediaKind $kind, string $language): string
    {
        return \sprintf('tmdb_%s_%s', $kind->tmdbPath(), $language);
    }

    /**
     * @param list<mixed> $stored
     *
     * @return list<TmdbHit>
     */
    private function hitsFrom(array $stored): array
    {
        $hits = [];
        foreach ($stored as $data) {
            $hit = \is_array($data) ? TmdbHit::fromArray($data) : null;
            if ($hit instanceof TmdbHit) {
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
