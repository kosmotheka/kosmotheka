<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Serializer\Attribute\SerializedName;
use Symfony\Component\Validator\Constraints as Assert;

/**
 * Body of `DELETE /v1/account`, authenticated (docs/architecture/accounts.md). The password, or for an account that has
 * none an `email_code` (asked for with `POST /v1/account/code/action`); and the `totp_code` if it has the second factor
 * on (D100).
 */
final readonly class DeleteAccountRequest
{
    public function __construct(
        #[Assert\Length(max: 4096)]
        public ?string $password = null,
        #[Assert\Regex('/^[0-9]{6}$/D')]
        #[SerializedName('email_code')]
        public ?string $emailCode = null,
        #[Assert\Length(max: 20)]
        #[SerializedName('totp_code')]
        public ?string $totpCode = null,
    ) {
    }
}
