<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\CloudLibrary;
use App\Entity\Membership;
use App\Entity\User;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\Persistence\ManagerRegistry;
use Symfony\Component\Uid\Uuid;

/**
 * @extends ServiceEntityRepository<CloudLibrary>
 */
final class CloudLibraryRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, CloudLibrary::class);
    }

    /**
     * The one way into a cloud library (D84): through the access table. The id
     * is what the client says; the membership is what decides.
     */
    public function findOneAccessibleBy(Uuid $id, User $user): ?CloudLibrary
    {
        $library = $this->createQueryBuilder('l')
            ->innerJoin(Membership::class, 'm', 'WITH', 'm.cloudLibrary = l')
            ->where('l.id = :id')
            ->andWhere('m.user = :user')
            ->setParameter('id', $id, 'uuid')
            ->setParameter('user', $user)
            ->getQuery()
            ->getOneOrNullResult();
        \assert(null === $library || $library instanceof CloudLibrary);

        return $library;
    }

    /**
     * @return list<CloudLibrary>
     */
    public function findAllAccessibleBy(User $user): array
    {
        return $this->createQueryBuilder('l')
            ->innerJoin(Membership::class, 'm', 'WITH', 'm.cloudLibrary = l')
            ->where('m.user = :user')
            ->setParameter('user', $user)
            ->orderBy('l.createdAt')
            ->getQuery()
            ->getResult();
    }
}
