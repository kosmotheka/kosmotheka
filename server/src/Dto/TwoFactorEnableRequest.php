<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Serializer\Attribute\SerializedName;
use Symfony\Component\Validator\Constraints as Assert;

/**
 * Body of `POST /v1/account/two-factor/enable`, authenticated (docs/architecture/accounts.md). Besides the code of the
 * authenticator, the password; or for an account that has none, an `email_code` (D100).
 */
final readonly class TwoFactorEnableRequest
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Length(max: 20)]
        public string $code,
        #[Assert\Length(max: 4096)]
        public ?string $password = null,
        #[Assert\Regex('/^[0-9]{6}$/D')]
        #[SerializedName('email_code')]
        public ?string $emailCode = null,
    ) {
    }
}
