<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\User;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\Persistence\ManagerRegistry;
use Symfony\Component\Security\Core\User\PasswordAuthenticatedUserInterface;
use Symfony\Component\Security\Core\User\PasswordUpgraderInterface;

/**
 * @extends ServiceEntityRepository<User>
 */
final class UserRepository extends ServiceEntityRepository implements PasswordUpgraderInterface
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, User::class);
    }

    public function findOneByEmail(string $email): ?User
    {
        return $this->findOneBy(['email' => $email]);
    }

    public function findOneByGoogleSub(string $sub): ?User
    {
        return $this->findOneBy(['googleSub' => $sub]);
    }

    /** Rehashes on login when the hasher's cost has changed since the password was set (milestone 7.1, part 2). */
    /**
     * Takes a TOTP timestep if no later or equal one was taken, in the database: of two requests with the same code at
     * once, one gets it (SEG-072). The entity in memory is left as it was, so a later flush does not write it back.
     */
    public function claimTotpTimestep(User $user, int $timestep): bool
    {
        return 1 === $this->getEntityManager()->createQuery(\sprintf(
            'UPDATE %s u SET u.totpLastTimestep = :step WHERE u.id = :id AND (u.totpLastTimestep IS NULL OR u.totpLastTimestep < :step)',
            User::class,
        ))->setParameter('step', $timestep)->setParameter('id', $user->getId(), 'uuid')->execute();
    }

    public function upgradePassword(PasswordAuthenticatedUserInterface $user, string $newHashedPassword): void
    {
        if (!$user instanceof User) {
            return;
        }

        $user->setPassword($newHashedPassword);
        $this->getEntityManager()->flush();
    }
}
