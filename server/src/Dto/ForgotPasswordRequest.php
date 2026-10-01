<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Validator\Constraints as Assert;

/** Body of `POST /v1/account/password/forgot` (docs/architecture/accounts.md). */
final readonly class ForgotPasswordRequest
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Email]
        #[Assert\Length(max: 180)]
        public string $email,
    ) {
    }
}
