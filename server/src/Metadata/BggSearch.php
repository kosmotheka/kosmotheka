<?php

declare(strict_types=1);

namespace App\Metadata;

use App\Entity\MetadataCacheEntry;
use App\Enum\BggScope;
use App\Repository\MetadataCacheRepository;
use App\Security\AccountException;
use App\Security\RequestLimiter;
use Doctrine\DBAL\Exception\UniqueConstraintViolationException;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;

/**
 * Searches board games (BoardGameGeek) and role-playing books (RPGGeek) by title for an account (7.8, D62): the cache
 * first, then the site, within what the server allows itself. What is kept is the result of a search under a
 * fingerprint of what was searched, never the text, and each detail under the site's own number; nothing says who
 * asked. A failure is never kept. The details of all the hits are asked in one call.
 */
final readonly class BggSearch
{
    public function __construct(
        private BggSource $source,
        private MetadataCacheRepository $cache,
        private EntityManagerInterface $entityManager,
        private RequestLimiter $limiter,
        private ClockInterface $clock,
        private ProviderGate $gate,
    ) {
    }

    /** The key a search is kept under: a hash of the scope and the text as it is asked, so the table does not hold what people look for. */
    public static function fingerprint(string $scope, string $query): string
    {
        return hash('sha256', $scope."\n".self::normalise($query));
    }

    /** Trimmed, with every run of blanks (non-breaking ones too) as one space, and lower case. */
    public static function normalise(string $query): string
    {
        return TextCleaner::normaliseQuery($query);
    }

    /**
     * @return list<array{external_id: int, title: string, year: ?int, cover_url: ?string, publisher: ?string, designers: list<string>, artists: list<string>, players_min: ?int, players_max: ?int, duration_minutes: ?int, min_age: ?int, expands: ?array{external_id: int, title: string}}>
     *
     * @throws AccountException with AccountErrorCode::MetadataQuotaExhausted or MetadataFailed
     */
    public function search(BggScope $scope, string $query, string $accountId): array
    {
        $now = $this->clock->now();
        $hits = $this->hits($scope, self::normalise($query), $accountId, $now);
        $details = $this->details($scope, $hits, $accountId, $now);
        // Stored once, at the end: a row another request stored meanwhile must not leave anything to do after it (SEG-055).
        $this->flush();

        $candidates = [];
        foreach ($hits as $hit) {
            $detail = $details[$hit->id] ?? new BggDetail();
            $candidates[] = ['external_id' => $hit->id, 'title' => $detail->title ?? $hit->title, 'year' => $hit->year] + \array_slice($detail->toArray(), 1);
        }

        return $candidates;
    }

    /**
     * @return list<BggHit>
     *
     * @throws AccountException
     */
    private function hits(BggScope $scope, string $query, string $accountId, \DateTimeImmutable $now): array
    {
        $code = self::fingerprint($scope->value, $query);
        $entry = $this->cache->findOneByProviderAndCode(MetadataCacheEntry::BGG_SEARCH, $code);
        if ($entry instanceof MetadataCacheEntry && $entry->isFreshAt($now)) {
            $stored = $entry->getPayload()['hits'] ?? [];

            return $this->hitsFrom(\is_array($stored) ? array_values($stored) : []);
        }

        $this->gate->requireOpen($scope->provider());
        $hits = $this->gate->call(
            $scope->provider(),
            fn () => $this->limiter->consumeBggCallOf($accountId),
            fn (): array => $this->source->search($scope, $query),
        );

        $this->remember($entry, MetadataCacheEntry::BGG_SEARCH, $code, [] !== $hits, ['hits' => array_map(static fn (BggHit $hit): array => $hit->toArray(), $hits)], $now);

        return $hits;
    }

    /**
     * The details of these hits: those kept, and asked for together, in one call, those that are not. A hit whose
     * detail cannot be had stays in the answer without it.
     *
     * @param list<BggHit> $hits
     *
     * @return array<int, BggDetail>
     */
    private function details(BggScope $scope, array $hits, string $accountId, \DateTimeImmutable $now): array
    {
        $provider = $scope->provider();
        $details = [];
        $missing = [];
        $entries = [];
        foreach ($hits as $hit) {
            $entry = $this->cache->findOneByProviderAndCode($provider, (string) $hit->id);
            if ($entry instanceof MetadataCacheEntry && $entry->isFreshAt($now)) {
                $details[$hit->id] = BggDetail::fromArray($entry->getPayload());
                continue;
            }
            $entries[$hit->id] = $entry;
            $missing[] = $hit->id;
        }
        if ([] === $missing) {
            return $details;
        }

        try {
            $this->gate->requireOpen($provider);
        } catch (AccountException) {
            // The site asked us to leave it alone: the search stands without its details.
            return $details;
        }

        $fetched = $this->source->details(
            $scope,
            $missing,
            function () use ($accountId): bool {
                try {
                    $this->limiter->consumeBggCallOf($accountId);
                } catch (AccountException) {
                    return false;
                }

                return true;
            },
            fn () => $this->gate->pause($provider),
        );
        foreach ($fetched as $id => $detail) {
            if (null === $detail) {
                // What it answered it has none of is kept as missing, for an hour: asking again in a moment gets the same (SEG-053).
                $this->remember($entries[$id] ?? null, $provider, (string) $id, false, [], $now);
                continue;
            }
            $details[$id] = $detail;
            $this->remember($entries[$id] ?? null, $provider, (string) $id, true, $detail->toArray(), $now);
        }

        return $details;
    }

    /**
     * @param list<mixed> $stored
     *
     * @return list<BggHit>
     */
    private function hitsFrom(array $stored): array
    {
        $hits = [];
        foreach ($stored as $data) {
            $hit = \is_array($data) ? BggHit::fromArray($data) : null;
            if ($hit instanceof BggHit) {
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
