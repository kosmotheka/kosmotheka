<?php

declare(strict_types=1);

namespace App\Dto;

use App\Security\AccountException;

/**
 * One entry of a push. The operation stays a \stdClass, as decoded from the
 * request: it is stored and signed exactly as received, and only afterwards
 * read as an OperationPayload (App\Sync\OperationReader).
 */
final readonly class PushedOperation
{
    public function __construct(
        public int $schemaVersion,
        public \stdClass $operation,
    ) {
    }

    /**
     * The body of the link route: one entry of a push, `{"schemaVersion": n, "operation": {…}}`.
     *
     * @throws AccountException when the body is not of that shape
     */
    public static function fromJson(string $json): self
    {
        try {
            $body = json_decode($json, false, flags: \JSON_THROW_ON_ERROR);
        } catch (\JsonException) {
            throw AccountException::invalidRequest();
        }
        if (!$body instanceof \stdClass || !\is_int($body->schemaVersion ?? null) || !($body->operation ?? null) instanceof \stdClass) {
            throw AccountException::invalidRequest();
        }

        return new self($body->schemaVersion, $body->operation);
    }
}
