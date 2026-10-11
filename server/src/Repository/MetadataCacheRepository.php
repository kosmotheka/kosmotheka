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

    /** Deletes what is past its age: found books after their period, missing ones after theirs. */
    public function deleteStaleBefore(\DateTimeImmutable $now): int
    {
        return (int) $this->getEntityManager()->createQuery(
            \sprintf('DELETE FROM %s e WHERE (e.found = true AND e.fetchedAt <= :found) OR (e.found = false AND e.fetchedAt <= :missing)', MetadataCacheEntry::class)
        )
            ->setParameter('found', $now->modify('-'.MetadataCacheEntry::FOUND_TTL))
            ->setParameter('missing', $now->modify('-'.MetadataCacheEntry::MISSING_TTL))
            ->execute();
    }
}
