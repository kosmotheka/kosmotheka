<?php

declare(strict_types=1);

namespace App\Sync;

/**
 * A library is founded by one key (D97, rule 1 of spec/README.md): its id derives from the public key of the device
 * that founded it and a nonce, so only whoever holds that key can found it, whatever date they put on the operation.
 * SHA-256 of the 32 bytes of the key followed by the 16 bytes of the nonce, the first 16 bytes with the version nibble
 * set to 8 and the variant bits to 10, written as a lowercase UUID. Port of `library_id` in crates/domain/src/founding.rs.
 */
final class LibraryId
{
    private const int NONCE_BYTES = 16;

    /**
     * @param string $publicKey Ed25519, base64url without padding
     * @param string $nonce     16 bytes, base64url without padding
     *
     * @return ?string null when the key is not 32 bytes or the nonce is not 16
     */
    public static function derive(string $publicKey, string $nonce): ?string
    {
        try {
            $key = sodium_base642bin($publicKey, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
            $salt = sodium_base642bin($nonce, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        } catch (\SodiumException) {
            return null;
        }
        if (\SODIUM_CRYPTO_SIGN_PUBLICKEYBYTES !== \strlen($key) || self::NONCE_BYTES !== \strlen($salt)) {
            return null;
        }
        $bytes = substr(hash('sha256', $key.$salt, true), 0, 16);
        $bytes[6] = \chr(\ord($bytes[6]) & 0x0F | 0x80);
        $bytes[8] = \chr(\ord($bytes[8]) & 0x3F | 0x80);
        $hex = bin2hex($bytes);

        return \sprintf('%s-%s-%s-%s-%s', substr($hex, 0, 8), substr($hex, 8, 4), substr($hex, 12, 4), substr($hex, 16, 4), substr($hex, 20, 12));
    }
}
