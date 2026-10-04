<?php

declare(strict_types=1);

namespace App\Dto;

use App\Enum\CatalogEntity;
use App\Enum\FieldType;

final readonly class FieldSpec
{
    /**
     * @param list<string>   $allowed    values accepted by a OneOf field
     * @param ?CatalogEntity $references the entity a Ref field points to (D89: an optional
     *                                   reference to a deleted row is applied as null)
     */
    public function __construct(
        public FieldType $type,
        public bool $required,
        public array $allowed = [],
        public ?CatalogEntity $references = null,
    ) {
    }
}
