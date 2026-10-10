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
    case NewerVersion = 'newer_version';
    /** Its previous operation is not stored here (yet), or is not an earlier one of the same author (D87). */
    case BrokenChain = 'broken_chain';
    /** Another operation of this library already holds its HLC: only a modified app signs two with one time. */
    case HlcTaken = 'hlc_taken';
    /** Dated more than the allowed clock drift ahead of the server's clock (SEG-028): an honest device never signs it. */
    case TooFarAhead = 'too_far_ahead';
}
