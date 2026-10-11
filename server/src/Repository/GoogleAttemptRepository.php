<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\GoogleAttempt;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\Persistence\ManagerRegistry;

/**
 * @extends ServiceEntityRepository<GoogleAttempt>
 */
final class GoogleAttemptRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, GoogleAttempt::class);
    }

    public function findOneByStateHash(string $stateHash): ?GoogleAttempt
    {
        return $this->findOneBy(['stateHash' => $stateHash]);
    }

    public function deleteExpiredBefore(\DateTimeImmutable $at): int
    {
        return (int) $this->getEntityManager()->createQuery(
            \sprintf('DELETE FROM %s a WHERE a.expiresAt < :at', GoogleAttempt::class)
        )->setParameter('at', $at)->execute();
    }
}
