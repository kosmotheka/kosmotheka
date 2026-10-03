<?php

declare(strict_types=1);

namespace App\Command;

use App\Entity\User;
use App\Repository\UserRepository;
use App\Security\TwoFactor;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Style\SymfonyStyle;

/** For someone who lost both their phone and their backup codes (D78). */
#[AsCommand(name: 'app:user:two-factor-disable', description: "Removes an account's second factor")]
final class UserTwoFactorDisableCommand extends Command
{
    public function __construct(
        private readonly UserRepository $userRepository,
        private readonly TwoFactor $twoFactor,
        private readonly EntityManagerInterface $entityManager,
    ) {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addArgument('email', InputArgument::REQUIRED, 'The account email');
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

        if (!$user->isTwoFactorEnabled()) {
            $io->error(\sprintf('%s does not have a second factor enabled.', $email));

            return Command::FAILURE;
        }

        $this->twoFactor->disable($user);
        $this->entityManager->flush();
        $io->success(\sprintf('Removed the second factor of %s.', $email));

        return Command::SUCCESS;
    }

    /** Arguments come back as mixed; `email` is a required string argument. */
    private function text(mixed $value): string
    {
        return \is_string($value) ? $value : '';
    }
}
