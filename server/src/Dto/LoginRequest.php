<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Serializer\Attribute\SerializedName;
use Symfony\Component\Validator\Constraints as Assert;

/** Body of `POST /v1/auth/login` (docs/architecture/accounts.md). */
final readonly class LoginRequest
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Email]
        public string $email,
        #[Assert\NotBlank]
        #[Assert\Length(max: 4096)]
        public string $password,
        #[Assert\NotBlank]
        #[Assert\Uuid]
        #[SerializedName('device_id')]
        public string $deviceId,
        #[Assert\NotBlank(normalizer: 'trim')]
        #[Assert\Length(max: 100, normalizer: 'trim')]
        #[SerializedName('device_name')]
        public string $deviceName,
        #[Assert\NotBlank]
        #[Assert\Length(max: 200)]
        #[SerializedName('public_key')]
        public string $publicKey,
        #[Assert\NotBlank]
        #[Assert\Length(max: 200)]
        public string $challenge,
        #[Assert\NotBlank]
        #[Assert\Length(max: 200)]
        public string $signature,
    ) {
    }
}
