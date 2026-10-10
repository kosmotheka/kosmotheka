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
        // In lowercase, as the device of an operation's HLC (SEG-029): one spelling per device, compared exactly.
        #[Assert\Regex(OperationPayload::DEVICE_ID_PATTERN, message: 'The device id must be a UUID in lowercase.')]
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
        /** Required only when the account has 2FA enabled (D78). */
        #[Assert\Length(max: 20)]
        #[SerializedName('totp_code')]
        public ?string $totpCode = null,
    ) {
    }
}
