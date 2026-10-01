<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Serializer\Attribute\SerializedName;
use Symfony\Component\Validator\Constraints as Assert;

/** Body of `POST /v1/account/password`, authenticated (docs/architecture/accounts.md). */
final readonly class ChangePasswordRequest
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Length(max: 4096)]
        #[SerializedName('current_password')]
        public string $currentPassword,
        #[Assert\Length(min: 8, max: 4096)]
        #[SerializedName('new_password')]
        public string $newPassword,
    ) {
    }
}
