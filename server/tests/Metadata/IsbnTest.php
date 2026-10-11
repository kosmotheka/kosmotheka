<?php

declare(strict_types=1);

namespace App\Tests\Metadata;

use App\Metadata\Isbn;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

final class IsbnTest extends TestCase
{
    /** @return iterable<string, array{string, string}> */
    public static function valid(): iterable
    {
        yield 'thirteen digits' => ['9788408176039', '9788408176039'];
        yield 'hyphens' => ['978-84-08-17603-9', '9788408176039'];
        yield 'spaces' => ['978 84 08 17603 9', '9788408176039'];
        yield 'ten, with an X' => ['840817603X', '9788408176039'];
        yield 'ten, lower-case x' => ['840817603x', '9788408176039'];
        yield 'ten, with hyphens' => ['84-08-17603-X', '9788408176039'];
        yield 'ten, no X' => ['0140328726', '9780140328721'];
        yield 'a 979' => ['9791032305690', '9791032305690'];
    }

    #[DataProvider('valid')]
    public function testItNormalisesToThirteenDigits(string $given, string $expected): void
    {
        self::assertSame($expected, Isbn::parse($given)->value);
    }

    /** @return iterable<string, array{string}> */
    public static function invalid(): iterable
    {
        yield 'empty' => [''];
        yield 'letters' => ['abcdefghij'];
        yield 'thirteen letters' => ['abcdefghijklm'];
        yield 'bad thirteen check' => ['9788408176038'];
        yield 'bad ten check' => ['0140328727'];
        yield 'twelve' => ['978840817603'];
        yield 'not a book prefix' => ['1234567890123'];
        yield 'a line break' => ["9788408176039\n"];
        yield 'dots' => ['..'];
        yield 'huge' => [str_repeat('9', 100_000)];
        yield 'an X in a thirteen' => ['978840817603X'];
        yield 'unicode digits' => ['９７８８４０８１７６０３９'];
        yield 'a null byte' => ["9788408176039\0"];
    }

    #[DataProvider('invalid')]
    public function testItRefusesWhatIsNotAnIsbn(string $given): void
    {
        $this->expectException(\InvalidArgumentException::class);

        Isbn::parse($given);
    }
}
