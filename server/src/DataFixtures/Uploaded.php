<?php

declare(strict_types=1);

namespace App\DataFixtures;

use App\Entity\CloudLibrary;

/** A library a fixture uploaded, with the operations it sent. */
final readonly class Uploaded
{
    /**
     * @param list<array<string, mixed>> $operations
     */
    public function __construct(
        public CloudLibrary $library,
        public array $operations,
    ) {
    }
}
