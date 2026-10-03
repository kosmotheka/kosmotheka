<?php

declare(strict_types=1);

namespace App\Dto;

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
}
