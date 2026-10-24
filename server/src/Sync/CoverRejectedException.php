<?php

declare(strict_types=1);

namespace App\Sync;

use App\Enum\CoverRejection;

/** The server will not keep or give a cover; `$reason` says why (D82). */
final class CoverRejectedException extends \RuntimeException
{
    public function __construct(public readonly CoverRejection $reason)
    {
        parent::__construct($reason->value);
    }
}
