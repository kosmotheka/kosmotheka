<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Validator\Constraints as Assert;

/**
 * Body of `PATCH /v1/account` (docs/architecture/accounts.md). Both fields
 * are optional: `null` means "leave it as it is".
 */
final readonly class UpdateAccountRequest
{
    public function __construct(
        #[Assert\NotBlank(allowNull: true, normalizer: 'trim')]
        #[Assert\Length(max: 100, normalizer: 'trim')]
        public ?string $name = null,
        #[Assert\Choice(choices: ['en', 'es'])]
        public ?string $locale = null,
    ) {
    }
}
