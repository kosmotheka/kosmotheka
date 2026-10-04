<?php

declare(strict_types=1);

namespace App\Sync;

/**
 * How this server talks to its Mercure hub (D75). Mercure is optional (D18): an empty
 * MERCURE_URL means no hub, and then nothing is published and no token is issued.
 */
final readonly class HubSettings
{
    public function __construct(
        /** The hub's internal URL. It is also the audience (`aud`) of every token it accepts. */
        public string $url,
        public string $subscriberKey,
        public string $issuer,
    ) {
    }

    public function isEnabled(): bool
    {
        return '' !== $this->url;
    }
}
