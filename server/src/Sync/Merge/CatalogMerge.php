<?php

declare(strict_types=1);

namespace App\Sync\Merge;

use App\Dto\OperationPayload;
use App\Entity\CloudLibrary;
use App\Enum\ChangeOutcome;

/**
 * Applies operations to the catalog of a cloud library under the merge rules of
 * spec/README.md (D92). Callers depend on this interface, so the implementation
 * (today DbalCatalogMerge) can change without touching them.
 *
 * It never opens or closes a transaction: the caller does.
 */
interface CatalogMerge
{
    /**
     * Applies an already validated operation. Each change is judged on its own: one that breaks a rule is
     * skipped and writes nothing, and the others still apply.
     *
     * @return list<ChangeOutcome> one per change, in order
     */
    public function apply(CloudLibrary $library, OperationPayload $operation): array;

    /** Writes what is pending and forgets what the merge held in memory. */
    public function flush(): void;

    /** Removes the whole catalog of the library and its bookkeeping, tombstones included. */
    public function clear(CloudLibrary $library): void;

    /** Drops what is pending and remembered without writing it: after a failure, so nothing of it reaches the next library. */
    public function reset(): void;
}
