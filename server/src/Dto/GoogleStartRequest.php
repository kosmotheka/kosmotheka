<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Serializer\Attribute\SerializedName;
use Symfony\Component\Validator\Constraints as Assert;

/** Body of `POST /v1/auth/google/start` (docs/architecture/accounts.md, «Continuar con Google»). */
final readonly class GoogleStartRequest
{
    public function __construct(
        /** SHA-256 of the secret the app keeps (the verifier), base64url without padding: 43 characters. */
        #[Assert\NotBlank]
        #[Assert\Regex('/^[A-Za-z0-9_-]{43}$/', message: 'The hash of the verifier must be 43 base64url characters.')]
        #[SerializedName('verifier_hash')]
        public string $verifierHash,
        /** The language a new account is made in. */
        #[Assert\Choice(choices: ['en', 'es'])]
        public string $locale = 'en',
    ) {
    }
}
