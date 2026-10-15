<?php

declare(strict_types=1);

namespace App\Tests\Metadata;

use App\Metadata\TextCleaner;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

final class TextCleanerTest extends TestCase
{
    /** @return iterable<string, array{string, string}> */
    public static function hidden(): iterable
    {
        yield 'right-to-left override' => ["Dune\u{202E}txt.exe", 'Dunetxt.exe'];
        yield 'C1 control' => ["Dune\u{0085}Messiah", 'Dune Messiah'];
        yield 'zero width space' => ["Du\u{200B}ne", 'Dune'];
        yield 'isolate' => ["\u{2067}Dune\u{2069}", 'Dune'];
        yield 'ASCII control' => ["Dune\x07", 'Dune'];
    }

    #[DataProvider('hidden')]
    public function testWhatAProviderHidesInATextIsTakenOut(string $sent, string $kept): void
    {
        self::assertSame($kept, TextCleaner::clean($sent, 100));
    }

    public function testLettersOfAnyLanguageStay(): void
    {
        self::assertSame('El niño y la garza — 君たちはどう生きるか', TextCleaner::clean('El niño y la garza — 君たちはどう生きるか', 100));
        // The non-joiner is part of how Persian is written.
        self::assertSame("می\u{200C}خواهم", TextCleaner::clean("می\u{200C}خواهم", 100));
    }
}
