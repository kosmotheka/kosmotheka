<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\MetadataCacheEntry;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\Persistence\ManagerRegistry;

/**
 * @extends ServiceEntityRepository<MetadataCacheEntry>
 */
final class MetadataCacheRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, MetadataCacheEntry::class);
    }

    public function findOneByProviderAndCode(string $provider, string $code): ?MetadataCacheEntry
    {
        return $this->findOneBy(['provider' => $provider, 'code' => $code]);
    }

    /** Deletes what is past its age: each kind of search after its own period, found books and missing ones after theirs. */
    public function deleteStaleBefore(\DateTimeImmutable $now): int
    {
        $searches = array_keys(MetadataCacheEntry::SEARCH_TTLS);
        $query = $this->getEntityManager()->createQuery(
            \sprintf(
                'DELETE FROM %1$s e WHERE (e.provider = :tmdb AND e.fetchedAt <= :tmdbAge) OR (e.provider = :rawg AND e.fetchedAt <= :rawgAge) OR (e.provider NOT IN (:searches) AND ((e.found = true AND e.fetchedAt <= :found) OR (e.found = false AND e.fetchedAt <= :missing)))',
                MetadataCacheEntry::class,
            )
        );

        return (int) $query
            ->setParameter('tmdb', MetadataCacheEntry::TMDB_SEARCH)
            ->setParameter('tmdbAge', $now->modify('-'.MetadataCacheEntry::SEARCH_TTLS[MetadataCacheEntry::TMDB_SEARCH]))
            ->setParameter('rawg', MetadataCacheEntry::RAWG_SEARCH)
            ->setParameter('rawgAge', $now->modify('-'.MetadataCacheEntry::SEARCH_TTLS[MetadataCacheEntry::RAWG_SEARCH]))
            ->setParameter('searches', $searches)
            ->setParameter('found', $now->modify('-'.MetadataCacheEntry::FOUND_TTL))
            ->setParameter('missing', $now->modify('-'.MetadataCacheEntry::MISSING_TTL))
            ->execute();
    }
}
