<?php

declare(strict_types=1);

namespace App\Sync;

use App\Enum\SignatureErrorCode;

final class InvalidSignatureException extends \RuntimeException
{
    public function __construct(public readonly SignatureErrorCode $reason)
    {
        parent::__construct(\sprintf('Invalid operation signature: %s.', $reason->value));
    }
}
