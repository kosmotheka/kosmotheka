<?php

declare(strict_types=1);

namespace App\Operation;

final readonly class FieldSpec
{
    /**
     * @param list<string> $allowed values accepted by a OneOf field
     */
    public function __construct(
        public FieldType $type,
        public bool $required,
        public array $allowed = [],
    ) {
    }
}
