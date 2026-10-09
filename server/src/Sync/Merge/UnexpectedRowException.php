<?php

declare(strict_types=1);

namespace App\Sync\Merge;

/**
 * The database returned a catalog row that does not have the shape the mapping promises.
 */
final class UnexpectedRowException extends \UnexpectedValueException
{
}
