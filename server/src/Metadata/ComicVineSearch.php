<?php

declare(strict_types=1);

namespace App\Metadata;

use App\Entity\MetadataCacheEntry;
use App\Enum\ComicVineResource;
use App\Repository\MetadataCacheRepository;
use App\Security\AccountException;
use App\Security\RequestLimiter;
use Doctrine\DBAL\Exception\UniqueConstraintViolationException;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;

/**
 * Searches comics by title for an account (7.8, D62): the cache first, then Comic Vine, within what the server allows
 * itself. What is kept is the result of a search under a fingerprint of what was searched, never the text, and each
 * detail under Comic Vine's own number; nothing says who asked. A failure is never kept.
 */
final readonly class ComicVineSearch
{
    /** Only the first results get a second call: a search costs up to four. */
    private const int DETAILS_OF = 3;

    public function __construct(
        private ComicVineSource $source,
        private MetadataCacheRepository $cache,
        private EntityManagerInterface $entityManager,
        private RequestLimiter $limiter,
        private ClockInterface $clock,
        private ProviderGate $gate,
    ) {
    }

    public static function fingerprint(string $query): string
    {
        return hash('sha256', "comic_vine\n".self::normalise($query));
    }

    public static function normalise(string $query): string
    {
        return TextCleaner::normaliseQuery($query);
    }

    /**
     * @return list<array{resource: string, external_id: int, title: ?string, series: ?string, number: ?string, year: ?int, publisher: ?string, cover_url: ?string, writers: list<string>, artists: list<string>}>
     *
     * @throws AccountException with AccountErrorCode::MetadataQuotaExhausted or MetadataFailed
     */
    public function search(string $query): array
    {
        $now = $this->clock->now();
        $hits = $this->hits(self::normalise($query), $now);
        $details = $this->details(\array_slice($hits, 0, self::DETAILS_OF), $now);
        // Stored once, at the end: a row another request stored meanwhile must not leave anything to do after it (SEG-055).
        $this->flush();

        $candidates = [];
        foreach ($hits as $hit) {
            $detail = $details[$this->key($hit)] ?? new ComicVineDetail();
            $candidates[] = $hit->toArray() + $detail->toArray();
        }

        return $candidates;
    }

    /**
     * @return list<ComicVineHit>
     *
     * @throws AccountException
     */
    private function hits(string $query, \DateTimeImmutable $now): array
    {
        $code = self::fingerprint($query);
        $entry = $this->cache->findOneByProviderAndCode(MetadataCacheEntry::COMIC_VINE_SEARCH, $code);
        if ($entry instanceof MetadataCacheEntry && $entry->isFreshAt($now)) {
            $stored = $entry->getPayload()['hits'] ?? [];

            return $this->hitsFrom(\is_array($stored) ? array_values($stored) : []);
        }

        $this->gate->requireOpen(ComicVineSource::NAME);
        $hits = $this->gate->call(
            ComicVineSource::NAME,
            fn () => $this->limiter->consumeComicVineCall(ComicVineResource::Search),
            fn (): array => $this->source->search($query),
        );

        $this->remember($entry, MetadataCacheEntry::COMIC_VINE_SEARCH, $code, [] !== $hits, ['hits' => array_map(static fn (ComicVineHit $hit): array => $hit->toArray(), $hits)], $now);

        return $hits;
    }

    /**
     * @param list<ComicVineHit> $hits
     *
     * @return array<string, ComicVineDetail>
     */
    private function details(array $hits, \DateTimeImmutable $now): array
    {
        $details = [];
        $missing = [];
        $entries = [];
        foreach ($hits as $hit) {
            $entry = $this->cache->findOneByProviderAndCode(self::detailProvider($hit->resource), (string) $hit->id);
            if ($entry instanceof MetadataCacheEntry && $entry->isFreshAt($now)) {
                $details[$this->key($hit)] = ComicVineDetail::fromArray($entry->getPayload());
                continue;
            }
            $entries[$this->key($hit)] = $entry;
            $missing[] = $hit;
        }
        if ([] === $missing) {
            return $details;
        }

        try {
            $this->gate->requireOpen(ComicVineSource::NAME);
        } catch (AccountException) {
            // Comic Vine asked us to leave it alone: the search stands without its details.
            return $details;
        }

        $fetched = $this->source->details(
            $missing,
            function (ComicVineResource $resource): bool {
                try {
                    $this->limiter->consumeComicVineCall($resource);
                } catch (AccountException) {
                    return false;
                }

                return true;
            },
            fn () => $this->gate->pause(ComicVineSource::NAME),
        );
        foreach ($missing as $hit) {
            $detail = $fetched[$this->key($hit)] ?? null;
            if (null === $detail) {
                continue;
            }
            $details[$this->key($hit)] = $detail;
            $this->remember($entries[$this->key($hit)] ?? null, self::detailProvider($hit->resource), (string) $hit->id, true, $detail->toArray(), $now);
        }

        return $details;
    }

    private function key(ComicVineHit $hit): string
    {
        return $hit->resource->value.':'.$hit->id;
    }

    public static function detailProvider(ComicVineResource $resource): string
    {
        return 'comic_vine_'.$resource->value;
    }

    /**
     * @param list<mixed> $stored
     *
     * @return list<ComicVineHit>
     */
    private function hitsFrom(array $stored): array
    {
        $hits = [];
        foreach ($stored as $data) {
            $hit = \is_array($data) ? ComicVineHit::fromArray($data) : null;
            if ($hit instanceof ComicVineHit) {
                $hits[] = $hit;
            }
        }

        return $hits;
    }

    /** @param array<string, mixed> $payload */
    private function remember(?MetadataCacheEntry $entry, string $provider, string $code, bool $found, array $payload, \DateTimeImmutable $now): void
    {
        if ($entry instanceof MetadataCacheEntry) {
            $entry->renew($found, $payload, $now);
        } else {
            $this->entityManager->persist(new MetadataCacheEntry($provider, $code, $found, $payload, $now));
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
