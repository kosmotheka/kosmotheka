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
        // One condition per kind of search, built from the one table of periods.
        $searchConditions = [];
        $parameters = [];
        foreach (MetadataCacheEntry::SEARCH_TTLS as $provider => $period) {
            $n = \count($parameters) / 2;
            $searchConditions[] = \sprintf('(e.provider = :provider%1$d AND e.fetchedAt <= :age%1$d)', $n);
            $parameters['provider'.$n] = $provider;
            $parameters['age'.$n] = $now->modify('-'.$period);
        }

        $query = $this->getEntityManager()->createQuery(
            \sprintf(
                'DELETE FROM %s e WHERE %s OR (e.provider NOT IN (:searches) AND ((e.found = true AND e.fetchedAt <= :found) OR (e.found = false AND e.fetchedAt <= :missing)))',
                MetadataCacheEntry::class,
                implode(' OR ', $searchConditions),
            )
        );
        foreach ($parameters as $name => $value) {
            $query->setParameter($name, $value);
        }

        return (int) $query
            ->setParameter('searches', array_keys(MetadataCacheEntry::SEARCH_TTLS))
            ->setParameter('found', $now->modify('-'.MetadataCacheEntry::FOUND_TTL))
            ->setParameter('missing', $now->modify('-'.MetadataCacheEntry::MISSING_TTL))
            ->execute();
    }
}
