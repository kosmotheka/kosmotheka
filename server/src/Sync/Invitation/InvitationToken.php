<?php

declare(strict_types=1);

namespace App\Sync\Invitation;

/**
 * The secret of an invitation link: 256 random bits in URL-safe base64. Only its SHA-256 is stored; being random,
 * the token needs no salt, and nobody can work back from the fingerprint to a link.
 */
final readonly class InvitationToken
{
    private const string FORMAT = '/^[A-Za-z0-9_-]{43}$/D';

    public static function generate(): string
    {
        return rtrim(strtr(base64_encode(random_bytes(32)), '+/', '-_'), '=');
    }

    /** Whether it could be a token we made: anything else is not worth a query. */
    public static function isWellFormed(string $token): bool
    {
        return 1 === preg_match(self::FORMAT, $token);
    }

    public static function fingerprint(string $token): string
    {
        return hash('sha256', $token);
    }
}
