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
        /** The provider answered, and what it answered is no use (a 404, a body that is not what it says it sends): asking again a moment later gets the same (SEG-053). */
        public readonly bool $answeredUnusable = false,
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

    public static function unusable(): self
    {
        return new self('The provider answered something unusable.', false, false, true);
    }

    /** It could not be asked, or did not finish answering. */
    public static function failed(): self
    {
        return new self('The provider could not be asked.', false);
    }
}
