<?php

declare(strict_types=1);

namespace App\DataFixtures;

/**
 * An attempt a fixture makes on purpose got through, or a normal case was
 * refused: a fixture that loads means the rules held, so this stops the load.
 */
final class RuleBrokenException extends \LogicException
{
}
