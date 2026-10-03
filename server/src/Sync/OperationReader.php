<?php

declare(strict_types=1);

namespace App\Sync;

use App\Dto\OperationPayload;
use App\Enum\OperationRejection;
use Symfony\Component\Serializer\Exception\ExceptionInterface;
use Symfony\Component\Serializer\SerializerInterface;
use Symfony\Component\Validator\Validator\ValidatorInterface;

/**
 * Reads a pushed operation as an OperationPayload, through the same Serializer
 * and ValidOperation constraint as any other operation, and adds what the
 * cloud needs on top.
 */
final readonly class OperationReader
{
    public function __construct(
        private SerializerInterface $serializer,
        private ValidatorInterface $validator,
    ) {
    }

    /**
     * @throws OperationRejectedException when it is not a valid operation
     */
    public function read(\stdClass $operation): OperationPayload
    {
        // The Serializer turns `{}` and `[]` into the same empty array, but only
        // `{}` is a set of fields; a list would later fail in the clients.
        if (!$this->hasObjectFields($operation)) {
            throw new OperationRejectedException(OperationRejection::Malformed);
        }

        try {
            $json = json_encode($operation, \JSON_THROW_ON_ERROR);
            $payload = $this->serializer->deserialize($json, OperationPayload::class, 'json', OperationPayload::SERIALIZATION_CONTEXT);
        } catch (\JsonException|ExceptionInterface) {
            throw new OperationRejectedException(OperationRejection::Malformed);
        }
        // PostgreSQL's JSONB cannot hold a NUL character.
        if (str_contains($json, '\u0000') || \count($this->validator->validate($payload)) > 0) {
            throw new OperationRejectedException(OperationRejection::Malformed);
        }

        return $payload;
    }

    private function hasObjectFields(\stdClass $operation): bool
    {
        $changes = $operation->changes ?? null;
        if (!\is_array($changes)) {
            return false;
        }

        return array_all($changes, static fn ($change): bool => $change instanceof \stdClass && ($change->fields ?? new \stdClass()) instanceof \stdClass);
    }
}
