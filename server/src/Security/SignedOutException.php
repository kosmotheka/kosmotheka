<?php

declare(strict_types=1);

namespace App\Security;

use App\Enum\SignOutReason;
use Symfony\Component\Security\Core\Exception\AuthenticationException;

/** A token the server signed out for a reason the app must show (D50). */
final class SignedOutException extends AuthenticationException
{
    public function __construct(public readonly SignOutReason $reason)
    {
        parent::__construct();
    }

    public function getMessageKey(): string
    {
        return 'This device was signed out by the server.';
    }
}
