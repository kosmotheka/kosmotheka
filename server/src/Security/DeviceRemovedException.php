<?php

declare(strict_types=1);

namespace App\Security;

use Symfony\Component\Security\Core\Exception\AuthenticationException;

/**
 * A token presented by a removed device (D81). Its own type so the
 * authenticator answers `device_removed` instead of `invalid_token`: the app
 * then wipes its libraries and starts again with a new identity.
 */
final class DeviceRemovedException extends AuthenticationException
{
    public function getMessageKey(): string
    {
        return 'This device was removed from its account.';
    }
}
