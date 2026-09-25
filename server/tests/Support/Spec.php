<?php

declare(strict_types=1);

namespace App\Tests\Support;

/**
 * The shared spec/ folder, at the root of the repository. In the container it
 * is mounted at /var/www/spec, which is the same relative position.
 */
final class Spec
{
    /**
     * @return array<mixed>
     */
    public static function json(string $relative): array
    {
        $path = \dirname(__DIR__, 3).'/spec/'.$relative;
        $contents = file_get_contents($path);
        if (false === $contents) {
            throw new \RuntimeException(\sprintf('Cannot read spec/%s', $relative));
        }
        $decoded = json_decode($contents, true, flags: \JSON_THROW_ON_ERROR);
        if (!\is_array($decoded)) {
            throw new \RuntimeException(\sprintf('spec/%s is not a JSON array or object', $relative));
        }

        return $decoded;
    }
}
