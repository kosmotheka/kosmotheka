<?php

declare(strict_types=1);

namespace App\Sync\Merge;

/**
 * A statement was asked for with a property the entity does not map. Nothing was run.
 */
final class UnknownPropertyException extends \LogicException
{
    public static function of(string $class, string $property): self
    {
        return new self(\sprintf('%s has no mapped property %s.', $class, json_encode($property, \JSON_INVALID_UTF8_SUBSTITUTE | \JSON_UNESCAPED_UNICODE)));
    }
}
