<?php

declare(strict_types=1);

namespace App\Sync;

/**
 * The id of the cloud device of a library derives from its public key (D95, rule 12 of spec/README.md): SHA-256 of
 * the 32 bytes of the key, the first 16 bytes, with the version nibble set to 8 and the variant bits to 10, written
 * as a lowercase UUID. A modified app can register any id with a key of its own, but not one whose derived id is
 * the cloud's, so the judge trusts the cloud device only when the key registered under its id gives that id.
 * Port of `cloud_device_id` in crates/domain/src/cloud.rs.
 */
final class CloudDeviceId
{
    /**
     * @param string $publicKey Ed25519, base64url without padding
     *
     * @return ?string null when it is not a key of 32 bytes
     */
    public static function fromPublicKey(string $publicKey): ?string
    {
        try {
            $key = sodium_base642bin($publicKey, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        } catch (\SodiumException) {
            return null;
        }
        if (\SODIUM_CRYPTO_SIGN_PUBLICKEYBYTES !== \strlen($key)) {
            return null;
        }
        $bytes = substr(hash('sha256', $key, true), 0, 16);
        $bytes[6] = \chr(\ord($bytes[6]) & 0x0F | 0x80);
        $bytes[8] = \chr(\ord($bytes[8]) & 0x3F | 0x80);
        $hex = bin2hex($bytes);

        return \sprintf('%s-%s-%s-%s-%s', substr($hex, 0, 8), substr($hex, 8, 4), substr($hex, 12, 4), substr($hex, 16, 4), substr($hex, 20, 12));
    }
}
