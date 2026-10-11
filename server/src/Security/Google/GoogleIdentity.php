<?php

declare(strict_types=1);

namespace App\Security\Google;

/**
 * What Google said about the person who just signed in, and nothing else: the identifier Google gives that person
 * (`sub`), the address, whether Google confirmed it, and a name. No token of Google's is kept past the exchange.
 */
final readonly class GoogleIdentity
{
    public function __construct(
        public string $sub,
        public string $email,
        public bool $emailVerified,
        public string $name,
    ) {
    }
}
