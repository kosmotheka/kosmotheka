<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\CloudLibrary;
use App\Entity\Membership;
use App\Entity\User;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\Persistence\ManagerRegistry;

/**
 * @extends ServiceEntityRepository<Membership>
 */
final class MembershipRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, Membership::class);
    }

    /** The account's row in the library, cut or not: the one door to a library asks for it (D84). */
    public function findOf(CloudLibrary $library, User $user): ?Membership
    {
        return $this->findOneBy(['cloudLibrary' => $library, 'user' => $user]);
    }

    /**
     * Every row of the account, cut ones included, oldest library first.
     *
     * @return list<Membership>
     */
    public function allOf(User $user): array
    {
        return $this->createQueryBuilder('m')->join('m.cloudLibrary', 'l')->where('m.user = :user')->setParameter('user', $user)->orderBy('l.createdAt')->getQuery()->getResult();
    }
}
