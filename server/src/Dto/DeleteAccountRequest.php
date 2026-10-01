<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Validator\Constraints as Assert;

/** Body of `DELETE /v1/account`, authenticated (docs/architecture/accounts.md). */
final readonly class DeleteAccountRequest
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Length(max: 4096)]
        public string $password,
    ) {
    }
}
