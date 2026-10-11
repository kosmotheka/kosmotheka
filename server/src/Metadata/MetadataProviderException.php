<?php

declare(strict_types=1);

namespace App\Metadata;

/**
 * A provider could not answer. The message is fixed text and there is never a `previous` exception: what the http
 * client throws can carry the address or a header, and the key must not travel into a log.
 */
final class MetadataProviderException extends \RuntimeException
{
    private function __construct(
        string $message,
        /** The provider said its quota is gone (or refused the key), not that it broke. */
        public readonly bool $quota,
    ) {
        parent::__construct($message);
    }

    public static function quota(): self
    {
        return new self('The provider refused the request: quota.', true);
    }

    public static function failed(): self
    {
        return new self('The provider could not be asked or answered something unusable.', false);
    }
}
