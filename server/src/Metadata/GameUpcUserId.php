<?php

declare(strict_types=1);

namespace App\Metadata;

use Symfony\Component\DependencyInjection\Attribute\Autowire;

/**
 * The user id GameUPC is sent with each choice (decided on 2026-10-23): it asked for one stable id per user, so that
 * it can undo the choices of somebody acting in bad faith without dropping everything sent with our key. It is a keyed
 * hash of the account's id, with a key derived from `APP_SECRET` for this purpose alone: the same account always gives
 * the same id, and nobody outside the server can tell whose it is or link it to anything else the server sends.
 */
final readonly class GameUpcUserId
{
    private string $key;

    public function __construct(
        #[Autowire(param: 'kernel.secret')]
        string $secret,
    ) {
        $this->key = hash_hmac('sha256', 'kosmotheka gameupc user', $secret, true);
    }

    /** 32 hexadecimal characters: GameUPC asks for at least 8. */
    public function of(string $accountId): string
    {
        return substr(hash_hmac('sha256', $accountId, $this->key), 0, 32);
    }
}
