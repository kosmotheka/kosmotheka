<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Serializer\Attribute\SerializedName;
use Symfony\Component\Validator\Constraints as Assert;

/** Body of `POST /v1/auth/google/finish`: the way back, plus the proof of the device that a login makes. */
final readonly class GoogleFinishRequest
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Length(max: 100)]
        public string $attempt,
        #[Assert\NotBlank]
        #[Assert\Length(max: 200)]
        public string $code,
        /** The secret the app kept since it started; the server only ever saw its hash. */
        #[Assert\NotBlank]
        #[Assert\Length(max: 200)]
        public string $verifier,
        #[Assert\NotBlank]
        #[Assert\Uuid]
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
        /** Required only when the account has a second factor: Google does not replace it (D78). */
        #[Assert\Length(max: 20)]
        #[SerializedName('totp_code')]
        public ?string $totpCode = null,
    ) {
    }
}
