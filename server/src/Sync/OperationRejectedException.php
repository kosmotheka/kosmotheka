<?php

declare(strict_types=1);

namespace App\Sync;

use App\Enum\OperationRejection;

/** One pushed operation is refused; the push goes on with the next. */
final class OperationRejectedException extends \RuntimeException
{
    public function __construct(public readonly OperationRejection $reason)
    {
        parent::__construct(\sprintf('Operation rejected: %s.', $reason->value));
    }
}
