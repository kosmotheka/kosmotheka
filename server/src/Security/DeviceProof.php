<?php

declare(strict_types=1);

namespace App\Security;

/**
 * Proves a login's device holds the private key for the public key it gives
 * (milestone 7.1, part 2, docs/architecture/accounts.md): the app signs
 * MESSAGE_FORMAT, filled with the challenge and the device id, with its
 * Ed25519 key. Challenge, public key and signature are base64url without
 * padding, decoded the way App\Sync\OperationSignature decodes a signature.
 */
final class DeviceProof
{
    public const string MESSAGE_FORMAT = "kosmotheka-login\n%s\n%s";

    /** @throws AccountException with AccountErrorCode::InvalidDeviceProof */
    public function verify(string $challenge, string $deviceId, string $publicKey, string $signature): void
    {
        $key = $this->decode($publicKey, \SODIUM_CRYPTO_SIGN_PUBLICKEYBYTES);
        $bytes = $this->decode($signature, \SODIUM_CRYPTO_SIGN_BYTES);
        $message = \sprintf(self::MESSAGE_FORMAT, $challenge, $deviceId);

        if (!sodium_crypto_sign_verify_detached($bytes, $message, $key)) {
            throw AccountException::invalidDeviceProof();
        }
    }

    private function decode(string $base64url, int $length): string
    {
        try {
            $bytes = sodium_base642bin($base64url, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        } catch (\SodiumException) {
            throw AccountException::invalidDeviceProof();
        }

        if ($length !== \strlen($bytes)) {
            throw AccountException::invalidDeviceProof();
        }

        return $bytes;
    }
}
