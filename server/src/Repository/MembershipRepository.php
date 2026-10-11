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
 * @extends ServiceEntityRepository<Membership>
 */
final class MembershipRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, Membership::class);
    }

    /** The account that is this person of the library, if one is: read from the server's own table, never from a client. */
    public function findOfMember(CloudLibrary $library, Uuid $member): ?Membership
    {
        return $this->findOneBy(['cloudLibrary' => $library, 'memberId' => $member, 'withdrawnAt' => null]);
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
