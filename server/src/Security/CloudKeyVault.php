<?php

declare(strict_types=1);

namespace App\Security;

/**
 * Encrypts the private key of each cloud library before it goes into the database (D95), with a secret
 * of the server that is not in the database: a copy of the database alone cannot sign. XSalsa20-Poly1305
 * (`sodium_crypto_secretbox`) with a random nonce per value, kept in front of the cipher text. Opening a
 * value with another secret, or one that was changed, fails; it never gives back bytes that are not the
 * original.
 */
final readonly class CloudKeyVault
{
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
     * @return string base64url without padding: nonce and cipher text
     *
     * @throws CloudKeyVaultException
     */
    public function seal(#[\SensitiveParameter] string $plain): string
    {
        $nonce = random_bytes(\SODIUM_CRYPTO_SECRETBOX_NONCEBYTES);

        return sodium_bin2base64($nonce.sodium_crypto_secretbox($plain, $nonce, $this->key()), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
    }

    /**
     * @return non-empty-string
     *
     * @throws CloudKeyVaultException when the value does not open with this secret
     */
    public function open(string $sealed): string
    {
        $key = $this->key();
        try {
            $bytes = sodium_base642bin($sealed, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        } catch (\SodiumException) {
            throw CloudKeyVaultException::cannotOpen();
        }
        if (\strlen($bytes) <= \SODIUM_CRYPTO_SECRETBOX_NONCEBYTES) {
            throw CloudKeyVaultException::cannotOpen();
        }
        $plain = sodium_crypto_secretbox_open(substr($bytes, \SODIUM_CRYPTO_SECRETBOX_NONCEBYTES), substr($bytes, 0, \SODIUM_CRYPTO_SECRETBOX_NONCEBYTES), $key);
        if (false === $plain || '' === $plain) {
            throw CloudKeyVaultException::cannotOpen();
        }

        return $plain;
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
