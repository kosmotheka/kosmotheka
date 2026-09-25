<?php

declare(strict_types=1);

use Rector\Config\RectorConfig;
use Rector\Doctrine\Set\DoctrineSetList;
use Rector\Set\ValueObject\SetList;
use Rector\Symfony\Set\SymfonySetList;

// withComposerBased() reads composer.json and applies the upgrade sets for the
// Doctrine and Symfony versions actually installed.
//
// No ->withSkip(): if a rule ever has to be turned off, it goes in with its
// reason, not on its own.
return RectorConfig::configure()
    ->withPaths([__DIR__.'/src', __DIR__.'/tests'])
    ->withPhpSets(php85: true)
    ->withComposerBased(doctrine: true, symfony: true)
    ->withImportNames(importDocBlockNames: true, importShortClasses: false, removeUnusedImports: true)
    ->withSets([
        SetList::CODE_QUALITY,
        SetList::DEAD_CODE,
        SetList::TYPE_DECLARATION,
        SymfonySetList::SYMFONY_CODE_QUALITY,
        DoctrineSetList::DOCTRINE_CODE_QUALITY,
        DoctrineSetList::TYPED_COLLECTIONS,
    ]);
