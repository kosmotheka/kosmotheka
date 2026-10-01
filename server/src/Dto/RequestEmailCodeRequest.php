<?php

declare(strict_types=1);

namespace App\Dto;

use App\Enum\EmailCodePurpose;
use Symfony\Component\Validator\Constraints as Assert;

/**
 * Body of `POST /v1/account/code`. Only `confirm_email` is accepted for now;
 * `reset_password` arrives through its own endpoint (part 3).
 */
final readonly class RequestEmailCodeRequest
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Email]
        public string $email,
        #[Assert\Choice(choices: ['confirm_email'])]
        public string $purpose,
    ) {
    }

    public function getPurpose(): EmailCodePurpose
    {
        return EmailCodePurpose::from($this->purpose);
    }
}
