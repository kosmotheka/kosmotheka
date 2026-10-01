<?php

declare(strict_types=1);

namespace App\Security;

use App\Entity\User;
use Symfony\Component\Security\Core\Authentication\Token\TokenInterface;
use Symfony\Component\Security\Core\User\UserCheckerInterface;
use Symfony\Component\Security\Core\User\UserInterface;

/**
 * Blocks a device-authenticated call, or a login (App\Controller\AuthController
 * calls it directly), for an account that is not usable (D49, D51). Deliberately
 * throws App\Security\AccountException, not an AuthenticationException: the
 * account API answers these the same way as any other business error, through
 * App\EventListener\AccountExceptionListener, not through an authentication
 * entry point.
 */
final class UserChecker implements UserCheckerInterface
{
    public function checkPreAuth(UserInterface $user): void
    {
        if (!$user instanceof User) {
            return;
        }

        if (!$user->isConfirmed()) {
            throw AccountException::accountNotConfirmed();
        }

        if ($user->isBlocked()) {
            throw AccountException::accountBlocked();
        }
    }

    public function checkPostAuth(UserInterface $user, ?TokenInterface $token = null): void
    {
    }
}
