<?php

declare(strict_types=1);

namespace App\Sync\Invitation;

use Symfony\Component\DependencyInjection\Attribute\Autowire;

/**
 * What the server keeps of the email an invitation is tied to (D43): a keyed hash of the normalised address, never
 * the address. An email is easy to guess and a plain hash would be tried against known addresses, so the key is a
 * secret of the server (derived from `APP_SECRET`, with its own purpose); losing it only voids the invitations
 * pending at that moment, which last 24 hours.
 */
final readonly class EmailFingerprint
{
    private string $key;

    public function __construct(
        #[Autowire(param: 'kernel.secret')]
        string $secret,
    ) {
        $this->key = hash_hmac('sha256', 'kosmotheka invitation email', $secret, true);
    }

    public function of(string $email): string
    {
        return hash_hmac('sha256', mb_strtolower(trim($email)), $this->key);
    }
}
