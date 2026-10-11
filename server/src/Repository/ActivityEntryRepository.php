<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\ActivityEntry;
use App\Enum\ActivityKind;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\Persistence\ManagerRegistry;
use Symfony\Component\Uid\Uuid;

/**
 * Reads the activity log, and deletes by age: nothing else ever changes it (D66).
 *
 * @extends ServiceEntityRepository<ActivityEntry>
 */
final class ActivityEntryRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, ActivityEntry::class);
    }

    /**
     * What `app:activity:list` shows, newest first.
     *
     * @return list<ActivityEntry>
     */
    public function search(?Uuid $account, ?Uuid $library, ?ActivityKind $kind, ?\DateTimeImmutable $since, ?\DateTimeImmutable $until, int $limit): array
    {
        $query = $this->createQueryBuilder('a')->orderBy('a.occurredAt', 'DESC')->addOrderBy('a.id', 'DESC')->setMaxResults($limit);
        if ($account instanceof Uuid) {
            $query->andWhere('a.accountId = :account')->setParameter('account', $account, 'uuid');
        }
        if ($library instanceof Uuid) {
            $query->andWhere('a.libraryId = :library')->setParameter('library', $library, 'uuid');
        }
        if ($kind instanceof ActivityKind) {
            $query->andWhere('a.kind = :kind')->setParameter('kind', $kind);
        }
        if ($since instanceof \DateTimeImmutable) {
            $query->andWhere('a.occurredAt >= :since')->setParameter('since', $since);
        }
        if ($until instanceof \DateTimeImmutable) {
            $query->andWhere('a.occurredAt < :until')->setParameter('until', $until);
        }

        /** @var list<ActivityEntry> $entries */
        $entries = $query->getQuery()->getResult();

        return $entries;
    }

    /** The purge: lines older than the cutoff, and no others. The only thing that ever deletes from the log. */
    public function deleteBefore(\DateTimeImmutable $cutoff): int
    {
        return (int) $this->getEntityManager()->createQuery(
            \sprintf('DELETE FROM %s a WHERE a.occurredAt < :cutoff', ActivityEntry::class)
        )->setParameter('cutoff', $cutoff)->execute();
    }
}
