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

    /** A string cleaned of control characters and of marks that hide or reverse text, cut to `$max` characters; null when it is not text, or empty. */
    public static function clean(mixed $value, int $max): ?string
    {
        if (!\is_string($value) || !mb_check_encoding($value, 'UTF-8')) {
            return null;
        }
        // Controls become a space; the marks that turn or hide text are taken out. The joiners some scripts need stay.
        $clean = (string) preg_replace('/[\x{200B}\x{202A}-\x{202E}\x{2066}-\x{2069}\x{FEFF}]+/u', '', $value);
        $clean = trim((string) preg_replace('/\p{Cc}+/u', ' ', $clean));

        return '' === $clean ? null : mb_substr($clean, 0, $max);
    }
}
