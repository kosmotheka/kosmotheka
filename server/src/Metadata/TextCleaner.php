<?php

declare(strict_types=1);

namespace App\Metadata;

/** What every provider's text goes through before it is kept or sent on: only text, no control characters, cut. */
final class TextCleaner
{
    /** What a person searched for, trimmed, with every run of blanks (non-breaking ones too) as one space, and lower case. */
    public static function normaliseQuery(string $query): string
    {
        return mb_strtolower(trim((string) preg_replace('/[\s\x{a0}]+/u', ' ', $query)));
    }

    /** A string cleaned of control characters and cut to `$max` characters; null when it is not text, or empty. */
    public static function clean(mixed $value, int $max): ?string
    {
        if (!\is_string($value) || !mb_check_encoding($value, 'UTF-8')) {
            return null;
        }
        $clean = trim((string) preg_replace('/[\x00-\x1F\x7F]+/', ' ', $value));

        return '' === $clean ? null : mb_substr($clean, 0, $max);
    }
}
