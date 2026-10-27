<?php

declare(strict_types=1);

namespace App\Enum;

enum FieldType
{
    case Text;
    case Integer;
    /** `YYYY-MM-DD`, and it must exist on the calendar. */
    case Date;
    /** A moment in UTC written one way only, `YYYY-MM-DDTHH:MM:SS.mmmZ`, that exists (D105). */
    case Timestamp;
    /** One of a fixed list of strings (FieldSpec::$allowed). */
    case OneOf;
    /** Id of another entity; the database enforces that it exists. */
    case Ref;
    /** `true` or `false`. */
    case Boolean;
    /** A whole number from 1 to 5. */
    case Rating;
    /** A whole number of at least 1. */
    case Count;
    /** Thirteen ASCII digits, without hyphens. */
    case Isbn13;
    /** The id of a device: a UUID in lowercase (SEG-029). */
    case DeviceId;
    /** The id of a device, or the empty text that takes it away. */
    case DeviceIdOrEmpty;
}
