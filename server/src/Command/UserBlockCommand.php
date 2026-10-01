<?php

declare(strict_types=1);

namespace App\Command;

use App\Entity\User;
use App\Repository\DeviceRepository;
use App\Repository\UserRepository;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Style\SymfonyStyle;

/** Blocking or unblocking an account is console-only, like ROLE_SUPER_ADMIN (D51). */
#[AsCommand(name: 'app:user:block', description: 'Blocks or unblocks an account')]
final class UserBlockCommand extends Command
{
    public function __construct(
        private readonly UserRepository $userRepository,
        private readonly DeviceRepository $deviceRepository,
        private readonly EntityManagerInterface $entityManager,
        private readonly ClockInterface $clock,
    ) {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this
            ->addArgument('email', InputArgument::REQUIRED, 'The account email')
            ->addOption('undo', null, InputOption::VALUE_NONE, 'Unblocks the account instead of blocking it');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $io = new SymfonyStyle($input, $output);
        $email = mb_strtolower(trim((string) $input->getArgument('email')));

        $user = $this->userRepository->findOneByEmail($email);
        if (!$user instanceof User) {
            $io->error(\sprintf('No account with email "%s".', $email));

            return Command::FAILURE;
        }

        if ($input->getOption('undo')) {
            $user->unblock();
            $this->entityManager->flush();
            $io->success(\sprintf('Unblocked %s.', $email));

            return Command::SUCCESS;
        }

        $user->block($this->clock->now());
        $this->deviceRepository->signOutAllExcept($user, null);
        $this->entityManager->flush();
        $io->success(\sprintf('Blocked %s and signed out all its devices.', $email));

        return Command::SUCCESS;
    }
}
