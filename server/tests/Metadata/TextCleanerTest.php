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

    /** @return iterable<string, array{mixed}> */
    public static function notWhole(): iterable
    {
        yield 'too long' => [str_repeat('a', 101)];
        yield 'a control' => ["Dune\x07"];
        yield 'a C1 control' => ["Dune\u{0085}Messiah"];
        yield 'right-to-left override' => ["Dune\u{202E}txt"];
        yield 'zero width space' => ["Du\u{200B}ne"];
        yield 'not UTF-8' => ["Dune\xFF"];
        yield 'only blanks' => ['   '];
        yield 'empty' => [''];
        yield 'not text' => [12];
    }

    #[DataProvider('notWhole')]
    public function testAValueThatDoesNotFitOrCarriesHiddenMarksIsDroppedWholeNotCut(mixed $value): void
    {
        self::assertNull(TextCleaner::whole($value, 100));
    }

    public function testAValueThatFitsComesOutAsItWasSent(): void
    {
        self::assertSame('Catán: 5 & 6 — 君', TextCleaner::whole('  Catán: 5 & 6 — 君 ', 100));
        self::assertSame(str_repeat('é', 100), TextCleaner::whole(str_repeat('é', 100), 100));
        self::assertSame("می\u{200C}خواهم", TextCleaner::whole("می\u{200C}خواهم", 100));
    }
}
