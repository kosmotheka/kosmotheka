<?php

declare(strict_types=1);

namespace App\Tests\Enum;

use App\Enum\Locale;
use PHPUnit\Framework\TestCase;

/** The list of languages is one: the validators, the account and TMDB all read it. */
final class LocaleTest extends TestCase
{
    public function testTheValuesAreTheCodesOfTheCases(): void
    {
        self::assertSame(['en', 'es'], Locale::values());
    }

    public function testAnUnknownCodeMeansTheFallback(): void
    {
        self::assertSame(Locale::FALLBACK, Locale::fromAny('fr'));
        self::assertSame(Locale::Es, Locale::fromAny('es'));
    }

    public function testEveryLocaleHasItsTmdbTag(): void
    {
        self::assertSame('es-ES', Locale::Es->tmdbTag());
        self::assertSame('en-US', Locale::FALLBACK->tmdbTag());
    }
}
