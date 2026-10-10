<?php

declare(strict_types=1);

namespace App\DataFixtures;

use App\Sync\OperationSignature;
use App\Sync\OperationSigner;
use Symfony\Component\Uid\Uuid;

/**
 * A device that builds operations the way the app does and signs them with the
 * same canonical bytes as App\Sync\OperationSignature. Fixtures and tests share
 * it, so both sign the same way. Time is the wall-clock part of the HLC, so the
 * caller controls the order.
 *
 * Only for dev and test: nothing in production builds operations.
 */
final readonly class SigningDevice
{
    /** The server the link operations name: the one the app of the fixture devices talks to, or it would not trust the link (D84). */
    public const string SERVER = CastFile::SERVER_URL;

    private function __construct(
        public string $id,
        public string $publicKey,
        private string $secretKey,
        /** The 32-byte seed of the key, hex; null when the key did not come from a seed. */
        public ?string $seed = null,
        /**
         * The id of the last operation this object signed, per library: the next one chains to it (D87).
         *
         * @var \ArrayObject<string, string>
         */
        private \ArrayObject $latest = new \ArrayObject(),
    ) {
    }

    /** A device with a fresh key; with an id of its own choosing, a stranger claiming someone else's id. */
    public static function generate(?string $id = null): self
    {
        return self::fromKeyPair($id ?? Uuid::v4()->toRfc4122(), sodium_crypto_sign_keypair());
    }

    /** The same id and key on every call: the key pair comes from a fixed 32-byte seed. */
    public static function fromSeed(string $id, string $seedName): self
    {
        $seed = hash('sha256', 'kosmotheka fixtures: '.$seedName, true);

        return self::fromKeyPair($id, sodium_crypto_sign_seed_keypair($seed), bin2hex($seed));
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

    /** The id of the last operation this object signed in the library: what the next one chains to (D87). */
    public function latest(string $library): ?string
    {
        return $this->latest[$library] ?? null;
    }

    /**
     * @return array<string, mixed>
     */
    public function registration(string $library, int $time, string $name = 'Test device', ?string $memberId = null, ?string $operationId = null): array
    {
        return $this->sign($library, $time, 'device.register', [[
            'entity' => 'device',
            'id' => $this->id,
            'action' => 'create',
            'fields' => ['public_key' => $this->publicKey, 'name' => $name] + (null === $memberId ? [] : ['member_id' => $memberId]),
        ]], $operationId);
    }

    /**
     * The operation that links a library to its cloud id (D84).
     *
     * @return array<string, mixed>
     */
    public function link(string $library, string $cloudLibraryId, int $time, ?string $operationId = null): array
    {
        return $this->sign($library, $time, 'library.link', [[
            'entity' => 'library',
            'id' => $library,
            'action' => 'update',
            'fields' => ['cloud_library_id' => $cloudLibraryId, 'cloud_server' => self::SERVER],
        ]], $operationId);
    }

    /**
     * @return array<string, mixed>
     */
    public function publisher(string $library, int $time, string $name = 'Minotauro', string|false|null $previous = false): array
    {
        return $this->sign($library, $time, 'publisher.add', [[
            'entity' => 'publisher',
            'id' => Uuid::v7()->toRfc4122(),
            'action' => 'create',
            'fields' => ['name' => $name],
        ]], previous: $previous);
    }

    /**
     * @param list<array<string, mixed>> $changes
     * @param string|null                $id       the operation's id; a fresh one when null
     * @param string|false|null          $previous the operation it chains to (D87): false for the last one this object signed in
     *                                             the library, null for none. A device of the story that signs in several steps
     *                                             names it, because every step builds the device again
     *
     * @return array<string, mixed>
     */
    public function sign(string $library, int $time, string $intent, array $changes, ?string $id = null, string|false|null $previous = false): array
    {
        $operation = [
            'id' => $id ?? Uuid::v7()->toRfc4122(),
            'library' => $library,
            'hlc' => \sprintf('%016d-%08x-%s', $time, 0, $this->id),
            'intent' => $intent,
            'changes' => $changes,
            'previous' => false === $previous ? ($this->latest[$library] ?? null) : $previous,
        ];
        $this->latest[$library] = $operation['id'];

        return $operation + ['signature' => $this->signature($operation)];
    }

    /**
     * Signs an operation built by hand as it stands, for tests that need a malformed one.
     *
     * @param array<string, mixed> $operation
     *
     * @return array<string, mixed>
     */
    public function resign(array $operation): array
    {
        unset($operation['signature']);

        return $operation + ['signature' => $this->signature($operation)];
    }

    /**
     * @param array<string, mixed> $operation
     */
    private function signature(array $operation): string
    {
        \assert('' !== $this->secretKey);

        return new OperationSigner(new OperationSignature())->sign($operation, $this->secretKey);
    }

    private static function fromKeyPair(string $id, string $keyPair, ?string $seed = null): self
    {
        \assert('' !== $keyPair);

        return new self($id, self::encode(sodium_crypto_sign_publickey($keyPair)), sodium_crypto_sign_secretkey($keyPair), $seed);
    }

    private static function encode(string $bytes): string
    {
        return sodium_bin2base64($bytes, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
    }
}
