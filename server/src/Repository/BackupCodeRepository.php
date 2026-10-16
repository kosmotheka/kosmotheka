<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\BackupCode;
use App\Entity\User;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\Persistence\ManagerRegistry;

/**
 * @extends ServiceEntityRepository<BackupCode>
 */
final class BackupCodeRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, BackupCode::class);
    }

    public function findOneUnusedByUserAndHash(User $user, string $codeHash): ?BackupCode
    {
        return $this->findOneBy(['user' => $user, 'codeHash' => $codeHash, 'usedAt' => null]);
    }

    /** Marks the code used if it was not yet, in the database: of two requests with it at once, one gets it (SEG-072). */
    public function markUsed(BackupCode $code, \DateTimeImmutable $at): bool
    {
        return 1 === $this->createQueryBuilder('b')
            ->update()
            ->set('b.usedAt', ':at')
            ->where('b.id = :id')
            ->andWhere('b.usedAt IS NULL')
            ->setParameter('at', $at, 'datetime_immutable')
            ->setParameter('id', $code->getId(), 'uuid')
            ->getQuery()
            ->execute();
    }

    public function countUnusedByUser(User $user): int
    {
        return (int) $this->createQueryBuilder('b')
            ->select('COUNT(b.id)')
            ->where('b.user = :user')
            ->andWhere('b.usedAt IS NULL')
            ->setParameter('user', $user)
            ->getQuery()
            ->getSingleScalarResult();
    }

    /** Regenerating backup codes (D78) replaces the old ten outright; a bulk delete, nothing needs them back in memory. */
    public function deleteAllByUser(User $user): void
    {
        $this->createQueryBuilder('b')
            ->delete()
            ->where('b.user = :user')
            ->setParameter('user', $user)
            ->getQuery()
            ->execute();
    }
}
