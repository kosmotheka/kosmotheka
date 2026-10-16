<?php

declare(strict_types=1);

namespace App\Tests\Sync;

use App\Sync\LibraryId;
use App\Sync\NoLibraryIdException;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

/** The derivation of D97, the same as `library_id` in crates/domain/src/founding.rs and the vector of spec/README.md. */
final class LibraryIdTest extends TestCase
{
    private const string KEY = 'iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w';
    private const string NONCE = 'AAECAwQFBgcICQoLDA0ODw';

    public function testTheIdComesFromTheKeyAndTheNonce(): void
    {
        self::assertSame('2c33de53-4834-8a05-b014-c42dc60bd4e1', LibraryId::derive(self::KEY, self::NONCE));
        self::assertNotSame(LibraryId::derive(self::KEY, self::NONCE), LibraryId::derive(self::KEY, 'AAECAwQFBgcICQoLDA0ODg'));
        self::assertNotSame(LibraryId::derive(self::KEY, self::NONCE), LibraryId::derive('jojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w', self::NONCE));
    }

    /** @param array{string, string} $arguments */
    #[DataProvider('notAKeyAndANonce')]
    public function testWhatIsNotAKeyAndANonceGivesNoId(array $arguments): void
    {
        $this->expectException(NoLibraryIdException::class);

        LibraryId::derive(...$arguments);
    }

    /** @return iterable<string, array{array{string, string}}> */
    public static function notAKeyAndANonce(): iterable
    {
        yield 'a short key' => [['short', self::NONCE]];
        yield 'a short nonce' => [[self::KEY, 'short']];
        yield 'no nonce' => [[self::KEY, '']];
        yield 'not base64' => [['not base64 !', self::NONCE]];
        yield 'a nonce of 17 bytes' => [[self::KEY, 'AAECAwQFBgcICQoLDA0ODxA']];
    }
}
