<?php

declare(strict_types=1);

namespace App\Security\Google;

/** Where the browser goes to sign in with Google, and the PKCE secret of that trip, which stays on the server. */
final readonly class GoogleAuthorization
{
    public function __construct(
        public string $url,
        public string $pkceVerifier,
    ) {
    }
}
