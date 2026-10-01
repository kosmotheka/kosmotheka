<?php

declare(strict_types=1);

namespace App\Command;

use App\Entity\User;
use App\Repository\UserRepository;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Style\SymfonyStyle;

/** ROLE_SUPER_ADMIN is granted only from here, never through the API (D51). */
#[AsCommand(name: 'app:user:super-admin', description: 'Grants or revokes ROLE_SUPER_ADMIN for an account')]
final class UserSuperAdminCommand extends Command
{
    public function __construct(
        private readonly UserRepository $userRepository,
        private readonly EntityManagerInterface $entityManager,
    ) {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this
            ->addArgument('email', InputArgument::REQUIRED, 'The account email')
            ->addOption('revoke', null, InputOption::VALUE_NONE, 'Revokes the role instead of granting it');
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

        $revoke = (bool) $input->getOption('revoke');
        $this->setSuperAdmin($user, !$revoke);
        $this->entityManager->flush();

        $io->success($revoke
            ? \sprintf('Revoked ROLE_SUPER_ADMIN from %s.', $email)
            : \sprintf('Granted ROLE_SUPER_ADMIN to %s.', $email));

        return Command::SUCCESS;
    }

    private function setSuperAdmin(User $user, bool $granted): void
    {
        // getRoles() always appends ROLE_USER; strip it before storing so it
        // is not persisted redundantly alongside the real custom roles.
        $roles = array_values(array_diff($user->getRoles(), ['ROLE_USER', 'ROLE_SUPER_ADMIN']));
        if ($granted) {
            $roles[] = 'ROLE_SUPER_ADMIN';
        }

        $user->setRoles($roles);
    }
}
