<?php

declare(strict_types=1);

namespace App\Tests\Support;

use App\Sync\OperationSignature;
use Symfony\Component\Uid\Uuid;

/**
 * A signing device for the cloud tests: builds operations the way the app does
 * and signs them with the same canonical bytes as App\Sync\OperationSignature.
 * Time is the wall-clock part of the HLC, so a test controls the order.
 */
final readonly class TestDevice
{
    public const string SERVER = 'https://kosmotheka.test';

    private function __construct(
        public string $id,
        public string $publicKey,
        private string $secretKey,
    ) {
    }

    /** A device with a fresh key; with an id of its own choosing, a stranger claiming someone else's id. */
    public static function generate(?string $id = null): self
    {
        $keyPair = sodium_crypto_sign_keypair();

        return new self(
            $id ?? Uuid::v4()->toRfc4122(),
            self::encode(sodium_crypto_sign_publickey($keyPair)),
            sodium_crypto_sign_secretkey($keyPair),
        );
    }

    /**
     * The device a signed-in account holds (DeviceAuthTestTrait).
     *
     * @param array{device_id: string, public_key: string, secret_key: string} $session
     */
    public static function fromSession(array $session): self
    {
        $secret = sodium_base642bin($session['secret_key'], \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);

        return new self($session['device_id'], $session['public_key'], $secret);
    }

    /**
     * @return array<string, mixed>
     */
    public function registration(string $library, int $time): array
    {
        return $this->sign($library, $time, 'device.register', [[
            'entity' => 'device',
            'id' => $this->id,
            'action' => 'create',
            'fields' => ['public_key' => $this->publicKey, 'name' => 'Test device'],
        ]]);
    }

    /**
     * The operation that links a library to its cloud id (D84).
     *
     * @return array<string, mixed>
     */
    public function link(string $library, string $cloudLibraryId, int $time): array
    {
        return $this->sign($library, $time, 'library.link', [[
            'entity' => 'library',
            'id' => $library,
            'action' => 'update',
            'fields' => ['cloud_library_id' => $cloudLibraryId, 'cloud_server' => self::SERVER],
        ]]);
    }

    /**
     * @return array<string, mixed>
     */
    public function publisher(string $library, int $time, string $name = 'Minotauro'): array
    {
        return $this->sign($library, $time, 'publisher.add', [[
            'entity' => 'publisher',
            'id' => Uuid::v7()->toRfc4122(),
            'action' => 'create',
            'fields' => ['name' => $name],
        ]]);
    }

    /**
     * @param list<array<string, mixed>> $changes
     *
     * @return array<string, mixed>
     */
    public function sign(string $library, int $time, string $intent, array $changes): array
    {
        $operation = [
            'id' => Uuid::v7()->toRfc4122(),
            'library' => $library,
            'hlc' => \sprintf('%016d-%08x-%s', $time, 0, $this->id),
            'intent' => $intent,
            'changes' => $changes,
        ];

        return $operation + ['signature' => $this->signature($operation)];
    }

    /**
     * @param array<string, mixed> $operation
     */
    private function signature(array $operation): string
    {
        $decoded = json_decode(json_encode($operation, \JSON_THROW_ON_ERROR), false, flags: \JSON_THROW_ON_ERROR);
        \assert($decoded instanceof \stdClass && '' !== $this->secretKey);
        $canonical = new OperationSignature()->canonical($decoded);

        return self::encode(sodium_crypto_sign_detached($canonical, $this->secretKey));
    }

    private static function encode(string $bytes): string
    {
        return sodium_bin2base64($bytes, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
    }
}
