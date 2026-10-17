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

    /**
     * A value of a provider whose terms forbid changing its data (BoardGameGeek): it comes out as it was sent, with only
     * the blanks around it trimmed, or not at all. One that is too long, is not UTF-8, or holds control characters or
     * marks that hide or reverse text is dropped whole, never cut or cleaned.
     */
    public static function whole(mixed $value, int $max): ?string
    {
        if (!\is_string($value) || !mb_check_encoding($value, 'UTF-8')) {
            return null;
        }
        $trimmed = trim($value);
        if ('' === $trimmed || mb_strlen($trimmed) > $max || 1 === preg_match('/[\p{Cc}\x{200B}\x{202A}-\x{202E}\x{2066}-\x{2069}\x{FEFF}]/u', $trimmed)) {
            return null;
        }

        return $trimmed;
    }
}
