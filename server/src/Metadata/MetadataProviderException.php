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
        /** The provider itself broke (a 5xx): the same question may get an answer a moment later. */
        public readonly bool $brokeItself = false,
    ) {
        parent::__construct($message);
    }

    public static function quota(): self
    {
        return new self('The provider refused the request: quota.', true);
    }

    public static function brokeItself(): self
    {
        return new self('The provider answered that it is broken.', false, true);
    }

    public static function failed(): self
    {
        return new self('The provider could not be asked or answered something unusable.', false);
    }
}
