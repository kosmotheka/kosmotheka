<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\CloudLibrary;
use App\Entity\Invitation;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\Persistence\ManagerRegistry;
use Symfony\Component\Uid\Uuid;

/**
 * @extends ServiceEntityRepository<Invitation>
 */
final class InvitationRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, Invitation::class);
    }

    public function findOneByTokenHash(string $hash): ?Invitation
    {
        return $this->findOneBy(['tokenHash' => $hash]);
    }

    /** One invitation of THIS library: an id from the client is compared against it, never looked up alone (D84). */
    public function findOneIn(CloudLibrary $library, Uuid $id): ?Invitation
    {
        return $this->findOneBy(['cloudLibrary' => $library, 'id' => $id]);
    }

    /** @return list<Invitation> newest first */
    public function allOf(CloudLibrary $library): array
    {
        return $this->findBy(['cloudLibrary' => $library], ['createdAt' => 'DESC']);
    }

    /** The purge: every invitation expired before the cutoff, whatever became of it (accepted, revoked or forgotten). */
    public function deleteExpiredBefore(\DateTimeImmutable $cutoff): int
    {
        return (int) $this->getEntityManager()->createQuery(
            \sprintf('DELETE FROM %s i WHERE i.expiresAt < :cutoff', Invitation::class)
        )->setParameter('cutoff', $cutoff)->execute();
    }
}
