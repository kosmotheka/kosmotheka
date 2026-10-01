<?php

declare(strict_types=1);

namespace App\Scheduler;

use App\Security\AccountPurger;
use Symfony\Component\Messenger\Attribute\AsMessageHandler;

#[AsMessageHandler]
final readonly class PurgeAccountsMessageHandler
{
    public function __construct(
        private AccountPurger $purger,
    ) {
    }

    public function __invoke(PurgeAccountsMessage $message): void
    {
        $this->purger->purge();
    }
}
