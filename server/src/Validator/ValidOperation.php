<?php

declare(strict_types=1);

namespace App\Validator;

use Symfony\Component\Validator\Constraint;

/**
 * The structural rules of spec/README.md. Reports only the first broken rule,
 * in the order the spec fixes, with its spec error code as the violation code.
 * Whether referenced rows exist, or whether the author may make the changes,
 * is checked when applying.
 */
#[\Attribute(\Attribute::TARGET_CLASS)]
final class ValidOperation extends Constraint
{
    public function getTargets(): string
    {
        return self::CLASS_CONSTRAINT;
    }
}
