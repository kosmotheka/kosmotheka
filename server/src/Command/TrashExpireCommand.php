<?php

declare(strict_types=1);

namespace App\Command;

use App\Sync\TrashExpirer;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Style\SymfonyStyle;

/** Runs App\Sync\TrashExpirer by hand; the daily run is the scheduler (docs/development/server.md). */
#[AsCommand(name: 'app:trash:expire', description: 'Deletes for good, signed as the cloud, what has been 60 days in the trash of the cloud libraries (D105)')]
final class TrashExpireCommand extends Command
{
    public function __construct(
        private readonly TrashExpirer $expirer,
    ) {
        parent::__construct();
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        new SymfonyStyle($input, $output)->success(\sprintf('Deleted for good %d thing(s) from the trash.', $this->expirer->expire()));

        return Command::SUCCESS;
    }
}
