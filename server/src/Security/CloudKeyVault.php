<?php

declare(strict_types=1);

namespace App\Security;

/**
 * Encrypts the private key of each cloud library before it goes into the database (D95), with a secret
 * of the server that is not in the database: a copy of the database alone cannot sign. Opening a value with
 * another secret, or one that was changed, fails; it never gives back bytes that are not the original.
 *
 * Since SEG-074 a value is `v1.` and then XChaCha20-Poly1305 with the id of its cloud library as associated data (the
 * key is derived from the secret with a purpose of its own): a sealed key copied into the row of another library does
 * not open there. Before that it was `sodium_crypto_secretbox` without a version, tied to nothing; those are still read,
 * so a server with values of both kinds works, and the migration `Version20261016120000` seals the old ones again.
 */
final readonly class CloudKeyVault
{
    private const string VERSION = 'v1.';
    private const string PURPOSE = 'kosmotheka cloud library key';

    public function __construct(
        /** 32 random bytes in base64; empty when the server has no cloud identity (D18). */
        #[\SensitiveParameter]
        private string $secret,
    ) {
    }

    public function isEnabled(): bool
    {
        return '' !== $this->secret;
    }

    /**
     * @param string $libraryId the id of the cloud library the key belongs to: it is part of what authenticates the value
     *
     * @return string `v1.` and base64url without padding: nonce and cipher text
     *
     * @throws CloudKeyVaultException
     */
    public function seal(string $libraryId, #[\SensitiveParameter] string $plain): string
    {
        $nonce = random_bytes(\SODIUM_CRYPTO_AEAD_XCHACHA20POLY1305_IETF_NPUBBYTES);
        $cipher = sodium_crypto_aead_xchacha20poly1305_ietf_encrypt($plain, self::VERSION.$libraryId, $nonce, $this->aeadKey());

        return self::VERSION.sodium_bin2base64($nonce.$cipher, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
    }

    /**
     * @return non-empty-string
     *
     * @throws CloudKeyVaultException when the value does not open with this secret for this library
     */
    public function open(string $libraryId, string $sealed): string
    {
        return str_starts_with($sealed, self::VERSION) ? $this->openCurrent($libraryId, $sealed) : $this->openLegacy($sealed);
    }

    /** Whether the value is of the old kind, which the migration seals again. */
    public static function isLegacy(string $sealed): bool
    {
        return !str_starts_with($sealed, self::VERSION);
    }

    /** @return non-empty-string */
    private function openCurrent(string $libraryId, string $sealed): string
    {
        $key = $this->aeadKey();
        $bytes = $this->decode(substr($sealed, \strlen(self::VERSION)), \SODIUM_CRYPTO_AEAD_XCHACHA20POLY1305_IETF_NPUBBYTES);
        $plain = sodium_crypto_aead_xchacha20poly1305_ietf_decrypt(substr($bytes, \SODIUM_CRYPTO_AEAD_XCHACHA20POLY1305_IETF_NPUBBYTES), self::VERSION.$libraryId, substr($bytes, 0, \SODIUM_CRYPTO_AEAD_XCHACHA20POLY1305_IETF_NPUBBYTES), $key);
        if (false === $plain || '' === $plain) {
            throw CloudKeyVaultException::cannotOpen();
        }

        return $plain;
    }

    /** @return non-empty-string */
    private function openLegacy(string $sealed): string
    {
        $key = $this->key();
        $bytes = $this->decode($sealed, \SODIUM_CRYPTO_SECRETBOX_NONCEBYTES);
        $plain = sodium_crypto_secretbox_open(substr($bytes, \SODIUM_CRYPTO_SECRETBOX_NONCEBYTES), substr($bytes, 0, \SODIUM_CRYPTO_SECRETBOX_NONCEBYTES), $key);
        if (false === $plain || '' === $plain) {
            throw CloudKeyVaultException::cannotOpen();
        }

        return $plain;
    }

    /** @return string at least one byte longer than the nonce */
    private function decode(string $text, int $nonceBytes): string
    {
        try {
            $bytes = sodium_base642bin($text, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        } catch (\SodiumException) {
            throw CloudKeyVaultException::cannotOpen();
        }
        if (\strlen($bytes) <= $nonceBytes) {
            throw CloudKeyVaultException::cannotOpen();
        }

        return $bytes;
    }

    /** The key of the new kind: the secret, run through a purpose of its own so it is not the key of anything else. */
    private function aeadKey(): string
    {
        return hash_hmac('sha256', self::PURPOSE, $this->key(), true);
    }

    /**
     * @return non-empty-string
     *
     * @throws CloudKeyVaultException
     */
    private function key(): string
    {
        if (!$this->isEnabled()) {
            throw CloudKeyVaultException::disabled();
        }
        try {
            $key = sodium_base642bin($this->secret, \SODIUM_BASE64_VARIANT_ORIGINAL);
        } catch (\SodiumException) {
            throw CloudKeyVaultException::badSecret();
        }
        if (\SODIUM_CRYPTO_SECRETBOX_KEYBYTES !== \strlen($key)) {
            throw CloudKeyVaultException::badSecret();
        }

        return $key;
    }
}
