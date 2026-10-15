<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Serializer\Attribute\SerializedName;
use Symfony\Component\Validator\Constraints as Assert;

/** Body of `POST /v1/account/password/reset` (docs/architecture/accounts.md). */
final readonly class ResetPasswordRequest
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
        /** Required when the account has the second factor on: the email alone is not enough (D100). */
        #[Assert\Length(max: 20)]
        #[SerializedName('totp_code')]
        public ?string $totpCode = null,
    ) {
    }
}
