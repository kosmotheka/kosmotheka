<?php

declare(strict_types=1);

namespace App\Security\Google;

use App\Entity\EmailCode;
use App\Entity\User;
use App\Enum\ActivityKind;
use App\Repository\UserRepository;
use App\Security\AccountException;
use App\Security\ActivityLog;
use App\Security\RegistrationGate;
use App\Security\UserChecker;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;

/**
 * Which account the person Google vouched for enters (D77): the one that already has their `sub`; else the one with
 * their address (the same person, the same account); else a new one, if this server makes accounts for that address.
 * Only ever called with what Google confirmed: an address Google does not confirm creates and links nothing.
 */
final readonly class GoogleAccounts
{
    public function __construct(
        private UserRepository $users,
        private UserChecker $userChecker,
        private RegistrationGate $registrationGate,
        private ActivityLog $activityLog,
        private EntityManagerInterface $entityManager,
        private ClockInterface $clock,
    ) {
    }

    /** @throws AccountException */
    public function resolve(GoogleIdentity $identity, string $locale): User
    {
        if (!$identity->emailVerified) {
            throw AccountException::googleEmailNotVerified();
        }
        $email = mb_strtolower(trim($identity->email));
        if ('' === $identity->sub || '' === $email || mb_strlen($email) > 180) {
            throw AccountException::googleFailed();
        }

        // Known by the identifier Google gives, not by an address that Google lets change.
        $linked = $this->users->findOneByGoogleSub($identity->sub);
        if ($linked instanceof User) {
            $this->userChecker->checkPreAuth($linked);

            return $linked;
        }

        $user = $this->users->findOneByEmail($email);
        if (!$user instanceof User) {
            return $this->create($identity, $email, $locale);
        }

        // Somebody else's Google account, with an address that this one used to have: not this account.
        if (null !== $user->getGoogleSub()) {
            throw AccountException::googleOtherAccount();
        }
        if ($user->isBlocked()) {
            throw AccountException::accountBlocked();
        }

        if (!$user->isConfirmed()) {
            // Maybe made by somebody else to keep a door: the person Google confirms is the owner now, and the password
            // that was put on it, and any code waiting, die with the door.
            $user->confirm($this->now());
            $user->clearPassword();
            $this->entityManager->createQuery(\sprintf('DELETE FROM %s c WHERE c.user = :user', EmailCode::class))
                ->setParameter('user', $user)->execute();
        }
        $user->setGoogleSub($identity->sub);
        $this->activityLog->record(ActivityKind::AccountLinkedWithGoogle, $user);
        $this->userChecker->checkPreAuth($user);

        return $user;
    }

    private function create(GoogleIdentity $identity, string $email, string $locale): User
    {
        if (!$this->registrationGate->allows($email)) {
            throw AccountException::registrationClosed();
        }

        $name = trim($identity->name);
        $user = new User($email, mb_substr('' !== $name ? $name : explode('@', $email)[0], 0, 100), $locale);
        $user->confirm($this->now());
        $user->setGoogleSub($identity->sub);
        $this->entityManager->persist($user);
        $this->activityLog->record(ActivityKind::AccountCreatedWithGoogle, $user);

        return $user;
    }

    private function now(): \DateTimeImmutable
    {
        return $this->clock->now()->setTimezone(new \DateTimeZone('UTC'));
    }
}
