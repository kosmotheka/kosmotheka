<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Validator\Constraints as Assert;

/** Body of `POST /v1/account/confirm` (docs/architecture/accounts.md). */
final readonly class ConfirmAccountRequest
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Email]
        public string $email,
        #[Assert\NotBlank]
        #[Assert\Regex('/^[0-9]{6}$/')]
        public string $code,
    ) {
    }
}
