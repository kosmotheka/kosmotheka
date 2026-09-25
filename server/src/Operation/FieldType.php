<?php

declare(strict_types=1);

namespace App\Operation;

enum FieldType
{
    case Text;
    case Integer;
    /** `YYYY-MM-DD`, and it must exist on the calendar. */
    case Date;
    /** One of a fixed list of strings (FieldSpec::$allowed). */
    case OneOf;
    /** Id of another entity; the database enforces that it exists. */
    case Ref;
}
