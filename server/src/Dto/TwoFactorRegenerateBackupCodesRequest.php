<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Validator\Constraints as Assert;

/**
 * Body of `POST /v1/account/two-factor/backup-codes`, authenticated
 * (docs/architecture/accounts.md). `code` is a TOTP code or a backup code,
 * with or without its dash.
 */
final readonly class TwoFactorRegenerateBackupCodesRequest
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Length(max: 4096)]
        public string $password,
        #[Assert\NotBlank]
        #[Assert\Length(max: 20)]
        public string $code,
    ) {
    }
}
