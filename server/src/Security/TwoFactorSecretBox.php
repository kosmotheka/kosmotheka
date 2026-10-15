<?php

declare(strict_types=1);

namespace App\Security;

use Symfony\Component\DependencyInjection\Attribute\Autowire;

/**
 * Seals the TOTP secret of an account before it goes into the database (SEG-063): a copy of the database alone must
 * not make the codes. The key comes from `APP_SECRET`, with its own purpose, like the backup codes' hash, so a server
 * without a cloud identity has it too; changing `APP_SECRET` voids the second factors, as it voids the backup codes.
 * XChaCha20-Poly1305 with the account's id as associated data: a sealed secret copied onto another account does not
 * open there.
 */
final readonly class TwoFactorSecretBox
{
    private const string VERSION = 'v1.';

    private string $key;

    public function __construct(
        #[Autowire(param: 'kernel.secret')]
        #[\SensitiveParameter]
        string $secret,
    ) {
        $this->key = self::keyFrom($secret);
    }

    public function seal(string $userId, #[\SensitiveParameter] string $plain): string
    {
        return self::sealWith($this->key, $userId, $plain);
    }

    /** @throws TwoFactorSecretException when the value is not one this server sealed for this account */
    public function open(string $userId, string $sealed): string
    {
        if (!str_starts_with($sealed, self::VERSION)) {
            throw TwoFactorSecretException::cannotOpen();
        }
        try {
            $bytes = sodium_base642bin(substr($sealed, \strlen(self::VERSION)), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        } catch (\SodiumException) {
            throw TwoFactorSecretException::cannotOpen();
        }
        $nonceBytes = \SODIUM_CRYPTO_AEAD_XCHACHA20POLY1305_IETF_NPUBBYTES;
        if (\strlen($bytes) <= $nonceBytes) {
            throw TwoFactorSecretException::cannotOpen();
        }
        $plain = sodium_crypto_aead_xchacha20poly1305_ietf_decrypt(substr($bytes, $nonceBytes), self::VERSION.$userId, substr($bytes, 0, $nonceBytes), $this->key);
        if (false === $plain || '' === $plain) {
            throw TwoFactorSecretException::cannotOpen();
        }

        return $plain;
    }

    /** For the migration that seals the secrets stored before this class existed. */
    public static function keyFrom(#[\SensitiveParameter] string $appSecret): string
    {
        return hash_hmac('sha256', 'kosmotheka two-factor secret', $appSecret, true);
    }

    public static function sealWith(#[\SensitiveParameter] string $key, string $userId, #[\SensitiveParameter] string $plain): string
    {
        $nonce = random_bytes(\SODIUM_CRYPTO_AEAD_XCHACHA20POLY1305_IETF_NPUBBYTES);

        return self::VERSION.sodium_bin2base64($nonce.sodium_crypto_aead_xchacha20poly1305_ietf_encrypt($plain, self::VERSION.$userId, $nonce, $key), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
    }
}
