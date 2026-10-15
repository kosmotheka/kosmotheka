<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Validator\Constraints as Assert;

/**
 * Body of `POST /v1/account/confirm` (docs/architecture/accounts.md). The password is chosen here, with the code, so
 * it is always the one of whoever has the email (D100).
 */
final readonly class ConfirmAccountRequest
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Email]
        public string $email,
        #[Assert\NotBlank]
        #[Assert\Regex('/^[0-9]{6}$/D')]
        public string $code,
        #[Assert\Length(min: 8, max: 4096)]
        public string $password,
    ) {
    }
}
