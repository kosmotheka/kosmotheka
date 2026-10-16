<?php

declare(strict_types=1);

namespace App\Tests\Sync;

use App\Sync\LibraryId;
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

    public function testWhatIsNotAKeyAndANonceGivesNoId(): void
    {
        self::assertNull(LibraryId::derive('short', self::NONCE));
        self::assertNull(LibraryId::derive(self::KEY, 'short'));
        self::assertNull(LibraryId::derive(self::KEY, ''));
        self::assertNull(LibraryId::derive('not base64 !', self::NONCE));
        self::assertNull(LibraryId::derive(self::KEY, 'AAECAwQFBgcICQoLDA0ODxA'), '17 bytes');
    }
}
