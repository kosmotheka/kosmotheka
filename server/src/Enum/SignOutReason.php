<?php

declare(strict_types=1);

namespace App\Enum;

/**
 * Why the server signed a device out, so the app can tell the person
 * (docs/design/errors.md). Signing out by hand needs no reason: the app
 * did it itself.
 */
enum SignOutReason: string
{
    case PasswordChanged = 'password_changed';
}
