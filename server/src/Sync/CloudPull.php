<?php

declare(strict_types=1);

namespace App\Sync;

use App\Dto\PullRequest;
use App\Entity\CloudLibrary;
use App\Repository\CloudOperationRepository;
use App\Security\AccountException;

/**
 * What a client is missing from one cloud library, by the version vector of
 * docs/architecture/sync.md: for each author, the operations after the last
 * one the client has. It says nothing about who is asking: the caller has
 * already checked access (App\Sync\CloudLibraryAccess).
 */
final readonly class CloudPull
{
    public function __construct(
        private SchemaVersion $schemaVersion,
        private CloudOperationRepository $operations,
    ) {
    }

    /**
     * @return array{operations: list<array{schemaVersion: int, operation: \stdClass}>, vector: \stdClass}
     *
     * @throws AccountException when the client is on another schema version
     */
    public function pull(CloudLibrary $library, PullRequest $request): array
    {
        $this->schemaVersion->require($request->schemaVersion);

        return [
            'operations' => $this->operations->findSince($library, $request->vector),
            // An object even when empty.
            'vector' => (object) $this->operations->vectorOf($library),
        ];
    }
}
