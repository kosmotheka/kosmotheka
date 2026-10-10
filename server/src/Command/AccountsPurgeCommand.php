<?php

declare(strict_types=1);

namespace App\Command;

use App\Security\AccountPurger;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Style\SymfonyStyle;

/** Runs App\Security\AccountPurger by hand; the hourly run is the scheduler (docs/development/server.md). */
#[AsCommand(name: 'app:accounts:purge', description: 'Deletes unconfirmed accounts older than 24h, expired email codes and invitations expired for a week')]
final class AccountsPurgeCommand extends Command
{
    public function __construct(
        private readonly AccountPurger $purger,
    ) {
        parent::__construct();
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $result = $this->purger->purge();

        new SymfonyStyle($input, $output)->success(\sprintf(
            'Deleted %d unconfirmed account(s), %d expired code(s) and %d old invitation(s).',
            $result['unconfirmed_users'],
            $result['expired_codes'],
            $result['invitations'],
        ));

        return Command::SUCCESS;
    }
}
