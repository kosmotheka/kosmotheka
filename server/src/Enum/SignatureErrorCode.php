<?php

declare(strict_types=1);

namespace App\Enum;

/**
 * Why an operation's signature is not accepted (spec/README.md, "Signatures").
 */
enum SignatureErrorCode: string
{
    case Missing = 'missing';
    case BadKey = 'bad_key';
    case Malformed = 'malformed';
    case Mismatch = 'mismatch';
}
