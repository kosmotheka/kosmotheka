<?php

declare(strict_types=1);

namespace App\DataFixtures;

use Doctrine\DBAL\Connection;
use Symfony\Component\DependencyInjection\Attribute\Autowire;

/**
 * Writes the cast as JSON once the fixtures are loaded, so the app's own tool
 * (app/src-tauri/examples/fixture_devices.rs) builds a device for each character
 * from the same source of truth. It holds secrets of fixture accounts only, in a
 * gitignored folder.
 */
final readonly class CastFile
{
    /** The nginx port of server/docker/docker-compose.yaml, as the host sees it. */
    public const string SERVER_URL = 'http://localhost:8095';

    public function __construct(
        private Connection $connection,
        #[Autowire(param: 'app.fixtures_cast_file')]
        private string $path,
    ) {
    }

    /** Returns the path it wrote. */
    public function write(): string
    {
        $byEmail = [];
        foreach ($this->connection->fetchAllAssociative('SELECT id, email, name, locale FROM "user"') as $row) {
            \assert(\is_string($row['id']) && \is_string($row['email']) && \is_string($row['name']) && \is_string($row['locale']));
            $byEmail[$row['email']] = ['id' => $row['id'], 'name' => $row['name'], 'locale' => $row['locale']];
        }

        $devices = [];
        foreach (Cast::devices() as $folder => ['character' => $character, 'email' => $email, 'device' => $device]) {
            $user = $byEmail[$email] ?? throw new \LogicException(\sprintf('The cast has no account %s in the database.', $email));
            $devices[] = [
                'folder' => $folder,
                'character' => $character,
                'email' => $email,
                'user' => ['id' => $user['id'], 'name' => $user['name'], 'locale' => $user['locale']],
                'deviceId' => $device->id,
                'signingSeed' => $device->seed,
                'token' => Cast::token($device),
                'server' => self::SERVER_URL,
            ];
        }

        if (!is_dir(\dirname($this->path)) && !mkdir(\dirname($this->path), 0o775, true) && !is_dir(\dirname($this->path))) {
            throw new \RuntimeException(\sprintf('Cannot create %s.', \dirname($this->path)));
        }
        file_put_contents($this->path, json_encode(['devices' => $devices], \JSON_PRETTY_PRINT | \JSON_UNESCAPED_SLASHES | \JSON_THROW_ON_ERROR)."\n");

        return $this->path;
    }
}
