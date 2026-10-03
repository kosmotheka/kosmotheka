<?php

declare(strict_types=1);

namespace App\Enum;

/**
 * Why a cloud library refused one pushed operation. Only what the operation
 * itself says counts (structure, signature, library), never the order in
 * which it arrived (D84, seam 7). The push reports counts, not reasons: a
 * refused operation is not sent again.
 */
enum OperationRejection: string
{
    case Malformed = 'malformed';
    case NotLinked = 'not_linked';
    case OtherLibrary = 'other_library';
    case UnknownKey = 'unknown_key';
    case KeyChanged = 'key_changed';
    case BadSignature = 'bad_signature';
}
