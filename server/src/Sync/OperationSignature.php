<?php

declare(strict_types=1);

namespace App\Sync;

use App\Enum\SignatureErrorCode;

/**
 * Port of crates/domain/src/signature.rs (D56): checks the Ed25519 signature of
 * an operation against its author's public key. Both run the cases in
 * spec/conformance/signatures.json.
 *
 * It works on the operation as decoded with objects as \stdClass (json_decode
 * without `true`), so an empty JSON object and an empty list stay different.
 */
final class OperationSignature
{
    /**
     * @throws InvalidSignatureException
     */
    public function verify(\stdClass $operation, string $publicKey): void
    {
        $signature = $operation->signature ?? null;
        if (!\is_string($signature)) {
            throw new InvalidSignatureException(SignatureErrorCode::Missing);
        }
        $key = $this->decode($publicKey, \SODIUM_CRYPTO_SIGN_PUBLICKEYBYTES, SignatureErrorCode::BadKey);
        $bytes = $this->decode($signature, \SODIUM_CRYPTO_SIGN_BYTES, SignatureErrorCode::Malformed);
        if (!sodium_crypto_sign_verify_detached($bytes, $this->canonical($operation), $key)) {
            throw new InvalidSignatureException(SignatureErrorCode::Mismatch);
        }
    }

    /**
     * The operation without its signature as canonical JSON (RFC 8785).
     */
    public function canonical(\stdClass $operation): string
    {
        $unsigned = clone $operation;
        unset($unsigned->signature);

        return $this->write($unsigned);
    }

    private function write(mixed $value): string
    {
        return match (true) {
            null === $value => 'null',
            true === $value => 'true',
            false === $value => 'false',
            \is_int($value) => (string) $value,
            \is_string($value) => $this->string($value),
            \is_array($value) => '['.implode(',', array_map($this->write(...), $value)).']',
            $value instanceof \stdClass => $this->object($value),
            default => throw new \InvalidArgumentException('Operations hold only strings, integers, booleans and nulls.'),
        };
    }

    private function object(\stdClass $object): string
    {
        $members = get_object_vars($object);
        // RFC 8785 orders keys by UTF-16 code units; comparing UTF-16BE bytes does that.
        uksort($members, static fn (string $a, string $b): int => strcmp(
            mb_convert_encoding($a, 'UTF-16BE', 'UTF-8'),
            mb_convert_encoding($b, 'UTF-16BE', 'UTF-8'),
        ));
        $parts = [];
        foreach ($members as $key => $member) {
            $parts[] = $this->string((string) $key).':'.$this->write($member);
        }

        return '{'.implode(',', $parts).'}';
    }

    private function string(string $text): string
    {
        $escaped = preg_replace_callback(
            '/[\x00-\x1f"\\\\]/',
            static fn (array $match): string => match ($match[0]) {
                '"' => '\"',
                '\\' => '\\\\',
                "\x08" => '\b',
                "\x0c" => '\f',
                "\n" => '\n',
                "\r" => '\r',
                "\t" => '\t',
                default => \sprintf('\u%04x', \ord($match[0])),
            },
            $text,
        );

        return '"'.$escaped.'"';
    }

    private function decode(string $base64url, int $length, SignatureErrorCode $error): string
    {
        try {
            $bytes = sodium_base642bin($base64url, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        } catch (\SodiumException) {
            throw new InvalidSignatureException($error);
        }
        if ($length !== \strlen($bytes)) {
            throw new InvalidSignatureException($error);
        }

        return $bytes;
    }
}
