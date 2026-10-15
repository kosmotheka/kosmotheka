<?php

declare(strict_types=1);

namespace App\Security;

/** A stored TOTP secret does not open: another `APP_SECRET`, a damaged value, or one sealed for another account. */
final class TwoFactorSecretException extends \RuntimeException
{
    public static function cannotOpen(): self
    {
        return new self('A second-factor secret does not open for this account with this APP_SECRET.');
    }
}
