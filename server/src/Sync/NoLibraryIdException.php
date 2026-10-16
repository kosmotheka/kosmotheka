<?php

declare(strict_types=1);

namespace App\Sync;

/** The key and the nonce are not a key and a nonce, so no library id derives from them (D97). */
final class NoLibraryIdException extends \RuntimeException
{
    public static function notAKeyAndANonce(): self
    {
        return new self('A library id derives from a 32-byte key and a 16-byte nonce, both base64url without padding.');
    }
}
