<?php

declare(strict_types=1);

namespace App\Sync;

/**
 * Signs an operation with an Ed25519 private key over the canonical bytes of OperationSignature (D56): the
 * counterpart of its verify(). The cloud signs with it (CloudAuthor); the fixtures and tests sign with it
 * too, so every signature is made the same way.
 */
final readonly class OperationSigner
{
    public function __construct(private OperationSignature $signature)
    {
    }

    /**
     * @param array<string, mixed> $operation without a signature
     * @param non-empty-string     $secretKey the 64-byte libsodium private key
     *
     * @return string the signature, base64url without padding
     */
    public function sign(array $operation, #[\SensitiveParameter] string $secretKey): string
    {
        unset($operation['signature']);
        // Objects as \stdClass, so an empty JSON object and an empty list stay different (OperationSignature).
        $decoded = json_decode(json_encode($operation, \JSON_THROW_ON_ERROR), false, flags: \JSON_THROW_ON_ERROR);
        \assert($decoded instanceof \stdClass);

        return sodium_bin2base64(sodium_crypto_sign_detached($this->signature->canonical($decoded), $secretKey), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
    }
}
