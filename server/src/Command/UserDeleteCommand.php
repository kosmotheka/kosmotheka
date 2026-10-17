<?php

declare(strict_types=1);

namespace App\Command;

use App\Entity\User;
use App\Repository\UserRepository;
use App\Security\AccountDeletion;
use App\Sync\CloudOwnership;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Style\SymfonyStyle;

/**
 * For whoever asks by email to have their account deleted: the same deletion as the app's (`AccountDeletion`), with the
 * hand-over of shared libraries (D99). Deleting rows by hand would skip it.
 */
#[AsCommand(name: 'app:user:delete', description: 'Deletes an account, handing over the libraries it uploaded as the app does')]
final class UserDeleteCommand extends Command
{
    public function __construct(
        private readonly UserRepository $userRepository,
        private readonly CloudOwnership $cloudOwnership,
        private readonly AccountDeletion $accountDeletion,
    ) {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this
            ->addArgument('email', InputArgument::REQUIRED, 'The account email')
            ->addOption('force', null, InputOption::VALUE_NONE, 'Does not ask for confirmation');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $io = new SymfonyStyle($input, $output);
        $email = mb_strtolower(trim($this->text($input->getArgument('email'))));

        $user = $this->userRepository->findOneByEmail($email);
        if (!$user instanceof User) {
            $io->error(\sprintf('No account with email "%s".', $email));

            return Command::FAILURE;
        }

        if (!$input->getOption('force')) {
            $this->describe($io, $user, $email);
            if (!$io->confirm('Delete this account? It cannot be undone.', false)) {
                $io->writeln('Nothing was deleted.');

                return Command::SUCCESS;
            }
        }

        $this->accountDeletion->delete($user, null);
        $io->success(\sprintf('Deleted the account %s.', $email));

        return Command::SUCCESS;
    }

    private function describe(SymfonyStyle $io, User $user, string $email): void
    {
        $plan = $this->cloudOwnership->previewFor($user);
        $io->writeln(\sprintf('Account: %s (%s)', $email, $user->getId()->toRfc4122()));
        if ([] === $plan) {
            $io->writeln('It uploaded no libraries to the cloud.');

            return;
        }
        $io->writeln(\sprintf('It uploaded %d cloud librar%s:', \count($plan), 1 === \count($plan) ? 'y' : 'ies'));
        foreach ($plan as ['library' => $library, 'to' => $to]) {
            $io->writeln(\sprintf(
                '  - %s: %s',
                $library->getId()->toRfc4122(),
                $to instanceof User ? 'passes to account '.$to->getId()->toRfc4122() : 'is deleted with the account (nobody else has a place in it)',
            ));
        }
    }

    /** Arguments come back as mixed; `email` is a required string argument. */
    private function text(mixed $value): string
    {
        return \is_string($value) ? $value : '';
    }
}
