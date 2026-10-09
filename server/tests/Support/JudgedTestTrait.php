<?php

declare(strict_types=1);

namespace App\Tests\Support;

use App\Entity\CloudLibrary;
use App\Repository\CloudOperationRepository;
use App\Sync\Permission\CatalogJudge;
use App\Sync\SchemaVersion;

/** For the kernel tests that deliver operations to the judge (see Judged). */
trait JudgedTestTrait
{
    private function judged(CloudLibrary $library): Judged
    {
        $container = self::getContainer();

        return new Judged($library, $container->get(CatalogJudge::class), $container->get(CloudOperationRepository::class), $container->get(SchemaVersion::class));
    }
}
