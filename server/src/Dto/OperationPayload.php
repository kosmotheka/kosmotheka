<?php

declare(strict_types=1);

namespace App\Dto;

use App\Validator\ValidOperation;
use Symfony\Component\Serializer\Normalizer\AbstractNormalizer;
use Symfony\Component\Validator\Constraints as Assert;

/**
 * An operation as it travels (spec/operation.schema.json).
 *
 * Two layers, as in the Rust client: the Serializer rejects anything that does
 * not have the shape of an operation (missing or unknown keys, wrong JSON
 * types, unknown entity or action), and ValidOperation applies the spec rules
 * with their error codes.
 */
#[ValidOperation]
final readonly class OperationPayload
{
    /**
     * Same strictness as the Rust client's `deny_unknown_fields`. Pass it to
     * #[MapRequestPayload(serializationContext: ...)] and to any denormalize().
     */
    public const array SERIALIZATION_CONTEXT = [AbstractNormalizer::ALLOW_EXTRA_ATTRIBUTES => false];

    /** Time, counter and device: `0001727222222222-00000003-<device id>`, the device being a UUID in lowercase (SEG-029). */
    public const string HLC_PATTERN = '/^[0-9]{16}-[0-9a-f]{8}-[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/D';

    /** The id of a device: a UUID written 8-4-4-4-12 in lowercase hexadecimal. Compared exactly, so one device has one writing (SEG-029). */
    public const string DEVICE_ID_PATTERN = '/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/D';

    /**
     * @param list<ChangePayload> $changes
     */
    public function __construct(
        public string $id,
        public string $library,
        #[Assert\Regex(self::HLC_PATTERN, message: 'Not a hybrid logical clock timestamp.')]
        public string $hlc,
        public string $intent,
        public array $changes,
        /** The latest operation of the same device in the same library when this one was made, null for the first (D87). The key is required. */
        public ?string $previous,
        /** Ed25519 signature of the author device (D56); none on operations from before phase 5. */
        #[Assert\Regex('/^[A-Za-z0-9_-]{86}$/D', message: 'Not an Ed25519 signature in base64url.')]
        public ?string $signature = null,
    ) {
    }
}
