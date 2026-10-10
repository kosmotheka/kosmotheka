<?php

declare(strict_types=1);

namespace App\Sync;

/** The cloud could not write an operation of its own: it has no identity, the library is not linked, or the log refused it. */
final class CloudAuthorException extends \RuntimeException
{
    public static function noIdentity(): self
    {
        return new self('This cloud library has no cloud device: the server has no CLOUD_KEY_SECRET, or the library had none when it was created.');
    }

    public static function notLinked(): self
    {
        return new self('The cloud signs only in a library that is linked to its local one.');
    }

    public static function noTransaction(): self
    {
        return new self('The cloud signs inside the transaction that holds the library lock, as a push does.');
    }

    public static function refused(): self
    {
        return new self('The log refused the operation the cloud signed.');
    }
}
