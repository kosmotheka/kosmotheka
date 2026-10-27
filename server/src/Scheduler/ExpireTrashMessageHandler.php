<?php

declare(strict_types=1);

namespace App\Scheduler;

use App\Sync\TrashExpirer;
use Symfony\Component\Messenger\Attribute\AsMessageHandler;

#[AsMessageHandler]
final readonly class ExpireTrashMessageHandler
{
    public function __construct(
        private TrashExpirer $expirer,
    ) {
    }

    public function __invoke(ExpireTrashMessage $message): void
    {
        $this->expirer->expire();
    }
}
