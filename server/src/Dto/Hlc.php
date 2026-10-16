<?php

declare(strict_types=1);

namespace App\Dto;

/** A hybrid logical clock timestamp, `<time>-<counter>-<device>`: the one place that reads the device out of it (CAL-006). */
final class Hlc
{
    /** The device that dated it, or '' when it is malformed: no device has that id, and the callers have validated it before. */
    public static function author(string $hlc): string
    {
        return explode('-', $hlc, 3)[2] ?? '';
    }
}
