<?php

declare(strict_types=1);

namespace App\Enum;

/** What the merge did with one change of an operation. */
enum ChangeOutcome
{
    /** The change wrote something: a row, a field, a version or a tombstone. */
    case Written;
    /** The change wrote nothing (a rule of the merge, not an error). */
    case Skipped;
}
