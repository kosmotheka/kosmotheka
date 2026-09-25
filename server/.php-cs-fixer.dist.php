<?php

declare(strict_types=1);

// Code style, with the ruleset the official Symfony repositories use:
// @Symfony + @Symfony:risky.
//
// How the three tools divide the work, so they do not step on each other:
//   - php-cs-fixer  → style (whitespace, import order, quotes)
//   - Rector        → syntax modernisation (promotion, types, PHP sets)
//   - PHPStan       → correctness (types, DQL, nulls)
//
// `declare(strict_types=1)` is enforced: this is an application, nobody calls
// these classes from loose code outside it.

$finder = (new PhpCsFixer\Finder())
    ->in([__DIR__.'/src', __DIR__.'/tests'])
    ->append([__DIR__.'/phpstan-em.php', __DIR__.'/rector.php', __FILE__]);

return (new PhpCsFixer\Config())
    ->setRiskyAllowed(true)
    ->setRules([
        '@Symfony' => true,
        '@Symfony:risky' => true,
        'declare_strict_types' => true,
        // Identity comparison with the literal on the left: Symfony's
        // convention, and it prevents accidental assignment inside an `if`.
        'yoda_style' => true,
    ])
    ->setFinder($finder);
