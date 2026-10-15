<?php

declare(strict_types=1);

namespace App\Security;

use App\Enum\DeviceProofPurpose;

/**
 * Proves a login's device holds the private key for the public key it gives
 * (milestone 7.1, part 2, docs/architecture/accounts.md): the app signs
 * MESSAGE_FORMAT, filled with this server's origin, what the signature is for,
 * the challenge and the device id, with its Ed25519 key. Challenge, public key
 * and signature are base64url without padding, decoded the way
 * App\Sync\OperationSignature decodes a signature. The old format, which named
 * neither the server nor the purpose, is not accepted (D100).
 */
final readonly class DeviceProof
{
    public const string MESSAGE_FORMAT = "kosmotheka-login-v2\n%s\n%s\n%s\n%s";

    public function __construct(private ServerOrigin $origin)
    {
    }

    /** @throws AccountException with AccountErrorCode::InvalidDeviceProof */
    public function verify(DeviceProofPurpose $purpose, string $challenge, string $deviceId, string $publicKey, string $signature): void
    {
        $key = $this->decode($publicKey, \SODIUM_CRYPTO_SIGN_PUBLICKEYBYTES);
        $bytes = $this->decode($signature, \SODIUM_CRYPTO_SIGN_BYTES);

        // The main address of the server or, in development, one of the others it is reached by; nothing else.
        foreach ($this->origin->accepted() as $origin) {
            $message = \sprintf(self::MESSAGE_FORMAT, $origin, $purpose->value, $challenge, $deviceId);
            if (sodium_crypto_sign_verify_detached($bytes, $message, $key)) {
                return;
            }
        }

        throw AccountException::invalidDeviceProof();
    }

    /** @return non-empty-string */
    private function decode(string $base64url, int $length): string
    {
        try {
            $bytes = sodium_base642bin($base64url, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        } catch (\SodiumException) {
            throw AccountException::invalidDeviceProof();
        }

        if ('' === $bytes || $length !== \strlen($bytes)) {
            throw AccountException::invalidDeviceProof();
        }

        return $bytes;
    }
}
