<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Validator\Constraints as Assert;

/**
 * Body of `DELETE /v1/account/two-factor`, authenticated (docs/architecture/
 * accounts.md). `code` is a TOTP code or a backup code, with or without its dash.
 */
final readonly class TwoFactorDisableRequest
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Length(max: 20)]
        public string $code,
        /** Asked for only when the account has one: one made with Google has none, and the code alone is the proof (D100). */
        #[Assert\Length(max: 4096)]
        public ?string $password = null,
    ) {
    }
}
