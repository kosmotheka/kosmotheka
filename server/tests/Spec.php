<?php

declare(strict_types=1);

namespace App\Tests;

/**
 * The shared spec/ folder, at the root of the repository. In the container it
 * is mounted at /var/www/spec, which is the same relative position.
 */
final class Spec
{
    public static function path(string $relative): string
    {
        return \dirname(__DIR__, 2).'/spec/'.$relative;
    }

    /**
     * @return array<mixed>
     */
    public static function json(string $relative): array
    {
        $contents = file_get_contents(self::path($relative));
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
