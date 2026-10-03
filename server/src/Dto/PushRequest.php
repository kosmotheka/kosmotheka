<?php

declare(strict_types=1);

namespace App\Dto;

use App\Security\AccountException;

/**
 * Body of `POST /v1/cloud/libraries/{id}/push`. Not bound with
 * #[MapRequestPayload]: one bad operation must be rejected on its own, not
 * fail the whole batch with a 422, and the Serializer cannot hand over
 * operations as \stdClass, which the signature needs to tell `{}` from `[]`.
 * Only the envelope is checked here; each operation is judged by the push.
 */
final readonly class PushRequest
{
    public const int MAX_OPERATIONS = 1000;

    /**
     * @param int                   $schemaVersion the client's own database version, like a pull's; the operations carry the one they were recorded at
     * @param list<PushedOperation> $operations
     */
    public function __construct(public int $schemaVersion, public array $operations)
    {
    }

    /**
     * @throws AccountException when the body is not `{"schemaVersion": n, "operations": [{"schemaVersion": n, "operation": {…}}]}`
     */
    public static function fromJson(string $json): self
    {
        try {
            $body = json_decode($json, false, flags: \JSON_THROW_ON_ERROR);
        } catch (\JsonException) {
            throw AccountException::invalidRequest();
        }
        if (!$body instanceof \stdClass || !\is_int($body->schemaVersion ?? null) || !\is_array($body->operations ?? null) || !array_is_list($body->operations)
            || \count($body->operations) > self::MAX_OPERATIONS) {
            throw AccountException::invalidRequest();
        }

        $operations = [];
        foreach ($body->operations as $entry) {
            if (!$entry instanceof \stdClass || !\is_int($entry->schemaVersion ?? null) || !($entry->operation ?? null) instanceof \stdClass) {
                throw AccountException::invalidRequest();
            }
            $operations[] = new PushedOperation($entry->schemaVersion, $entry->operation);
        }

        return new self($body->schemaVersion, $operations);
    }
}
