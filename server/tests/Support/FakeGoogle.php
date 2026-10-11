<?php

declare(strict_types=1);

namespace App\Tests\Support;

use App\Security\Google\GoogleAuthorization;
use App\Security\Google\GoogleIdentity;
use App\Security\Google\GoogleSignIn;
use App\Security\Google\GoogleSignInException;

/**
 * Google, in memory. It answers `CODE` with whoever `$identity` is, only if the PKCE secret it handed out comes
 * back with it, and counts what it was asked.
 */
final class FakeGoogle implements GoogleSignIn
{
    public const string CODE = 'google-code';

    // Static, because the test client boots a new kernel (and so a new service) for every request.
    public static bool $configured = false;
    public static ?GoogleIdentity $identity = null;
    public static int $asked = 0;
    /** @var list<string> the states it was given */
    public static array $states = [];
    private static string $pkce = '';

    public static function reset(): void
    {
        self::$configured = false;
        self::$identity = null;
        self::$asked = 0;
        self::$states = [];
        self::$pkce = '';
    }

    public function isConfigured(): bool
    {
        return self::$configured;
    }

    public function authorization(string $state): GoogleAuthorization
    {
        self::$states[] = $state;
        self::$pkce = 'pkce-'.bin2hex(random_bytes(8));

        return new GoogleAuthorization('https://accounts.google.test/auth?state='.rawurlencode($state), self::$pkce);
    }

    public function identify(string $code, string $pkceVerifier): GoogleIdentity
    {
        ++self::$asked;
        if (self::CODE !== $code || $pkceVerifier !== self::$pkce || !self::$identity instanceof GoogleIdentity) {
            throw new GoogleSignInException('Google refused.');
        }

        return self::$identity;
    }
}
