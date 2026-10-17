<?php

declare(strict_types=1);

namespace App\Enum;

/**
 * The one list of languages an account can have, and the app can be in: the server validates the `locale` of an account
 * against it and asks TMDB in it. A new language is a new case here, its `emails.<code>.yaml` in `translations/`, and
 * the matching file of the app (docs: «Añadir un idioma»).
 */
enum Locale: string
{
    case En = 'en';
    case Es = 'es';

    /** What an account or a search gets when its language is not in the list. */
    public const self FALLBACK = self::En;

    /** @return list<string> the codes, for `#[Assert\Choice(callback: …)]` */
    public static function values(): array
    {
        return array_map(static fn (self $locale): string => $locale->value, self::cases());
    }

    /** The language of a text that may be anything, as one of the list (the fallback when it is not). */
    public static function fromAny(string $code): self
    {
        return self::tryFrom($code) ?? self::FALLBACK;
    }

    /** How TMDB writes this language. */
    public function tmdbTag(): string
    {
        return match ($this) {
            self::En => 'en-US',
            self::Es => 'es-ES',
        };
    }
}
