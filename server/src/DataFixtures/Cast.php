<?php

declare(strict_types=1);

namespace App\DataFixtures;

use Symfony\Component\Uid\Uuid;

/**
 * Who is in the fixtures (docs/development/fixtures.md): fixed accounts,
 * devices, members and local library ids, so the database is the same on every
 * load and the tests can name its characters.
 */
final class Cast
{
    public const string PASSWORD = 'fixtures-only';

    public const string ANA = 'ana@garcia.test';
    public const string HUGO = 'hugo@garcia.test';
    public const string KENJI = 'kenji@tanaka.test';
    public const string JHONNY = 'jhonny@example.test';

    /** A local library id that looks random: the Garcías' own. */
    public const string GARCIA_LOCAL = '0199a3b2-7c4e-7d10-9a52-6f3e81c0b4d7';

    /** The fixed id of «Mi biblioteca» (D46): every old install has it. */
    public const string DEFAULT_LOCAL = '00000000-0000-7000-8000-000000000001';

    /** Another local id, for operations that do not belong to the library they are pushed to. */
    public const string OTHER_LOCAL = '00000000-0000-7000-8000-000000000002';

    /** The UUID v5 namespace of every fixed id below; a fixture-only value. */
    private const string NAMESPACE = '3d6f0a52-8b1e-4c47-a9d3-5e2c71f08b64';

    /** The wall-clock part of every HLC starts here (milliseconds). */
    private const int EPOCH_MS = 1_758_000_000_000;

    /** A fixed id for a name: the same UUID on every load. */
    public static function id(string $name): string
    {
        return Uuid::v5(Uuid::fromString(self::NAMESPACE), $name)->toRfc4122();
    }

    /** The HLC time of a moment in the story, in minutes after the start. */
    public static function at(int $minutes): int
    {
        return self::EPOCH_MS + $minutes * 60_000;
    }

    /** The bearer token of a fixture device, so a dev client or a test can call the API as it. */
    public static function token(SigningDevice $device): string
    {
        return sodium_bin2base64(hash('sha256', 'kosmotheka fixtures token: '.$device->id.$device->publicKey, true), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
    }

    public static function anaLaptop(): SigningDevice
    {
        return SigningDevice::fromSeed(self::id('device:ana-laptop'), 'ana laptop');
    }

    public static function anaPhone(): SigningDevice
    {
        return SigningDevice::fromSeed(self::id('device:ana-phone'), 'ana phone');
    }

    public static function hugoPhone(): SigningDevice
    {
        return SigningDevice::fromSeed(self::id('device:hugo-phone'), 'hugo phone');
    }

    public static function kenjiLaptop(): SigningDevice
    {
        return SigningDevice::fromSeed(self::id('device:kenji-laptop'), 'kenji laptop');
    }

    public static function jhonnyPhone(): SigningDevice
    {
        return SigningDevice::fromSeed(self::id('device:jhonny-phone'), 'jhonny phone');
    }

    /** Jhonny's key under the id of Ana's phone (SEG-016): the id is public, the key is his. */
    public static function jhonnyAsAnaPhone(): SigningDevice
    {
        return SigningDevice::fromSeed(self::anaPhone()->id, 'jhonny as ana phone');
    }

    /** Jhonny's key under the id of Ana's laptop. */
    public static function jhonnyAsAnaLaptop(): SigningDevice
    {
        return SigningDevice::fromSeed(self::anaLaptop()->id, 'jhonny as ana laptop');
    }

    /**
     * Every device of the cast that holds a session, by the name its data folder
     * gets in the app (docs/development/fixtures.md): who it belongs to and its key.
     *
     * @return array<string, array{character: string, email: string, device: SigningDevice}>
     */
    public static function devices(): array
    {
        return [
            'ana-laptop' => ['character' => 'Ana', 'email' => self::ANA, 'device' => self::anaLaptop()],
            'ana-phone' => ['character' => 'Ana', 'email' => self::ANA, 'device' => self::anaPhone()],
            'hugo-phone' => ['character' => 'Hugo', 'email' => self::HUGO, 'device' => self::hugoPhone()],
            'kenji-laptop' => ['character' => 'Kenji', 'email' => self::KENJI, 'device' => self::kenjiLaptop()],
            'jhonny-phone' => ['character' => 'Jhonny', 'email' => self::JHONNY, 'device' => self::jhonnyPhone()],
            'jhonny-as-ana-phone' => ['character' => 'Jhonny', 'email' => self::JHONNY, 'device' => self::jhonnyAsAnaPhone()],
        ];
    }
}
