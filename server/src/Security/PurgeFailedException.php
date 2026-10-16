<?php

declare(strict_types=1);

namespace App\Security;

/** Some steps of the hourly purge failed; the others ran (CAL-008). The first failure is the previous exception. */
final class PurgeFailedException extends \RuntimeException
{
    /** @param list<string> $steps */
    public function __construct(
        public readonly array $steps,
        \Throwable $previous,
    ) {
        parent::__construct(\sprintf('The purge failed in: %s.', implode(', ', $steps)), previous: $previous);
    }
}
