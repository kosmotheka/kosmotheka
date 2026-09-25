<?php

declare(strict_types=1);

namespace App\Dto;

use App\Enum\CatalogEntity;
use App\Enum\ChangeAction;

/**
 * One change of an operation, as it travels. Its rules live in
 * App\Validator\ValidOperation, which checks them in the order the spec fixes.
 */
final readonly class ChangePayload
{
    /**
     * @param array<string, mixed> $fields
     */
    public function __construct(
        public CatalogEntity $entity,
        public string $id,
        public ChangeAction $action,
        public array $fields = [],
    ) {
    }
}
