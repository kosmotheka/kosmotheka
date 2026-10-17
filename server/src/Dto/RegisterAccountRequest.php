<?php

declare(strict_types=1);

namespace App\Dto;

use App\Enum\Locale;
use Symfony\Component\Validator\Constraints as Assert;

/** Body of `POST /v1/account/register` (docs/architecture/accounts.md). */
final readonly class RegisterAccountRequest
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Email]
        #[Assert\Length(max: 180)]
        public string $email,
        #[Assert\Length(min: 8, max: 4096)]
        public string $password,
        #[Assert\NotBlank(normalizer: 'trim')]
        #[Assert\Length(max: 100, normalizer: 'trim')]
        public string $name,
        #[Assert\Choice(callback: [Locale::class, 'values'])]
        public string $locale,
    ) {
    }
}
