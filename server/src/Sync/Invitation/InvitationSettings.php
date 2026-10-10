<?php

declare(strict_types=1);

namespace App\Sync\Invitation;

use Symfony\Component\DependencyInjection\Attribute\Autowire;

/**
 * The address the invitation links start with (D79): `kosmotheka.app` in the official server, whatever the person
 * who hosts it sets in `INVITATION_BASE_URL` otherwise. Empty means the server makes no invitations (D18).
 */
final readonly class InvitationSettings
{
    private string $baseUrl;

    public function __construct(
        #[Autowire(env: 'INVITATION_BASE_URL')]
        string $baseUrl,
    ) {
        $this->baseUrl = rtrim($baseUrl, '/');
    }

    public function isEnabled(): bool
    {
        return '' !== $this->baseUrl;
    }

    /** What the links start with, so the app can tell which links are this server's; null when there are none. */
    public function baseUrl(): ?string
    {
        return $this->isEnabled() ? $this->baseUrl : null;
    }

    public function linkFor(string $token): string
    {
        return $this->baseUrl.'/i/'.$token;
    }
}
