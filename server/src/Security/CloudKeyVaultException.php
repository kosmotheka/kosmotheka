<?php

declare(strict_types=1);

namespace App\Security;

/**
 * The vault of the cloud's private keys could not do its job: no secret is set, the secret is not 32 bytes
 * of base64, or a stored value does not open with it (wrong secret or damaged value). Never returns bytes
 * that look like a key but are not.
 */
final class CloudKeyVaultException extends \RuntimeException
{
    public static function disabled(): self
    {
        return new self('CLOUD_KEY_SECRET is empty: the cloud has no identity on this server.');
    }

    public static function badSecret(): self
    {
        return new self('CLOUD_KEY_SECRET must be 32 random bytes in base64.');
    }

    public static function cannotOpen(): self
    {
        return new self('A cloud key does not open with this CLOUD_KEY_SECRET.');
    }
}
