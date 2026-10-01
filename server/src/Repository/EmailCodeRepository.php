<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\EmailCode;
use App\Entity\User;
use App\Enum\EmailCodePurpose;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\Persistence\ManagerRegistry;

/**
 * @extends ServiceEntityRepository<EmailCode>
 */
final class EmailCodeRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, EmailCode::class);
    }

    public function findOneByUserAndPurpose(User $user, EmailCodePurpose $purpose): ?EmailCode
    {
        return $this->findOneBy(['user' => $user, 'purpose' => $purpose]);
    }
}
