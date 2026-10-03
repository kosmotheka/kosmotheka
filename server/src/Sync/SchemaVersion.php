<?php

declare(strict_types=1);

namespace App\Sync;

use App\Security\AccountException;
use Symfony\Component\DependencyInjection\Attribute\Autowire;

/**
 * The database version devices and the server must share to sync, read from
 * spec/sync.json (D31). A client on another version gets a 409 and does not sync.
 */
final class SchemaVersion
{
    private ?int $current = null;

    public function __construct(
        #[Autowire('%kernel.project_dir%/../spec/sync.json')]
        private readonly string $specFile,
    ) {
    }

    public function current(): int
    {
        return $this->current ??= $this->read();
    }

    /**
     * @throws AccountException when the client's version is not the server's
     */
    public function require(int $clientVersion): void
    {
        if ($clientVersion !== $this->current()) {
            throw AccountException::schemaVersionMismatch();
        }
    }

    private function read(): int
    {
        $contents = file_get_contents($this->specFile);
        if (false === $contents) {
            throw new \RuntimeException(\sprintf('Cannot read %s.', $this->specFile));
        }
        $spec = json_decode($contents, true, flags: \JSON_THROW_ON_ERROR);
        if (!\is_array($spec) || !\is_int($spec['schemaVersion'] ?? null)) {
            throw new \RuntimeException(\sprintf('%s has no integer schemaVersion.', $this->specFile));
        }

        return $spec['schemaVersion'];
    }
}
