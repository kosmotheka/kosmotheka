<?php

declare(strict_types=1);

namespace App\Operation;

final readonly class OperationError
{
    public function __construct(
        public OperationErrorCode $code,
        /** Index of the offending change, if the problem is inside one. */
        public ?int $change = null,
        public ?string $field = null,
    ) {
    }

    public function inChange(int $index): self
    {
        return new self($this->code, $index, $this->field);
    }
}
