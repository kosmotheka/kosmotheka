<?php

declare(strict_types=1);

namespace App\Security;

use App\Entity\User;
use Symfony\Component\HttpFoundation\Request;

/**
 * The keys of the `#[RateLimit]` attributes on the controllers (config/packages/rate_limiter.yaml): each one is
 * passed as a first-class callable, `key: RateLimitKey::ip(...)`. The framework calls it with the action's resolved
 * arguments by name, so the account is the one every action asks for as `#[CurrentUser] User $user`.
 */
final class RateLimitKey
{
    /**
     * Per address; the real one when `TRUSTED_PROXIES` says so.
     *
     * @param array<string, mixed> $args
     */
    public static function ip(array $args, Request $request): string
    {
        return (string) $request->getClientIp();
    }

    /**
     * Per account.
     *
     * @param array<string, mixed> $args
     */
    public static function account(array $args): string
    {
        return self::accountId($args);
    }

    /**
     * Uploads have a bucket of their own, apart from downloads.
     *
     * @param array<string, mixed> $args
     */
    public static function accountPush(array $args): string
    {
        return self::accountId($args).'|push';
    }

    /** @param array<string, mixed> $args */
    public static function accountPull(array $args): string
    {
        return self::accountId($args).'|pull';
    }

    /** @param array<string, mixed> $args */
    private static function accountId(array $args): string
    {
        $user = $args['user'] ?? null;
        if (!$user instanceof User) {
            throw new \LogicException('A limit per account needs the action to take `#[CurrentUser] User $user`.');
        }

        return $user->getId()->toRfc4122();
    }
}
