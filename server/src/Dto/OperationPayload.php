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

    /**
     * @param list<ChangePayload> $changes
     */
    public function __construct(
        public string $id,
        #[Assert\Regex('/^[0-9]{16}-[0-9a-f]{8}-.+$/', message: 'Not a hybrid logical clock timestamp.')]
        public string $hlc,
        public string $intent,
        public array $changes,
    ) {
    }
}
