<?php

declare(strict_types=1);

namespace App\Tests\Support;

use Symfony\Component\Mercure\HubInterface;
use Symfony\Component\Mercure\Jwt\TokenFactoryInterface;
use Symfony\Component\Mercure\ProtocolVersion;
use Symfony\Component\Mercure\Update;

/**
 * Stands in for the Mercure hub in the test environment (config/services.yaml): it records
 * what would be published, or fails on demand. The state is static because the test client
 * reboots the kernel on every request, and with it every service.
 */
final class RecordingHub implements HubInterface
{
    /** @var list<Update> */
    public static array $updates = [];

    public static bool $failing = false;

    public static function reset(): void
    {
        self::$updates = [];
        self::$failing = false;
    }

    public function getPublicUrl(): string
    {
        return 'http://mercure.test/.well-known/mercure';
    }

    public function getFactory(): ?TokenFactoryInterface
    {
        return null;
    }

    public function publish(Update $update): string
    {
        if (self::$failing) {
            throw new \RuntimeException('The hub is down.');
        }
        self::$updates[] = $update;

        return 'urn:uuid:test';
    }

    public function getProtocolVersion(): ProtocolVersion
    {
        return ProtocolVersion::V1;
    }

    public function getCookieName(): string
    {
        return 'test';
    }
}
