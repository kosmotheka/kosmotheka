<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Validator\Constraints as Assert;

/** Body of `POST /v1/account/two-factor/enable`, authenticated (docs/architecture/accounts.md). */
final readonly class TwoFactorEnableRequest
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Length(max: 20)]
        public string $code,
    ) {
    }
}
