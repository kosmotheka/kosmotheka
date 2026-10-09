<?php

declare(strict_types=1);

namespace App\Command;

use App\Entity\CloudLibrary;
use App\Repository\CloudOperationRepository;
use App\Sync\Permission\CatalogJudge;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Style\SymfonyStyle;
use Symfony\Component\Uid\Uuid;

/**
 * Rebuilds the catalog of a cloud library from its operation log (milestones 7.4.4 and 7.4.5), judging each operation:
 * the log is the record, the catalog is derived from it.
 */
#[AsCommand(name: 'app:catalog:rebuild', description: 'Rebuilds the catalog of a cloud library from its operation log')]
final class CatalogRebuildCommand extends Command
{
    public function __construct(
        private readonly EntityManagerInterface $entityManager,
        private readonly CatalogJudge $judge,
        private readonly CloudOperationRepository $operations,
    ) {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addArgument('cloud-library', InputArgument::REQUIRED, 'The id of the cloud library (the server\'s own)');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $io = new SymfonyStyle($input, $output);
        $id = $input->getArgument('cloud-library');
        if (!\is_string($id) || !Uuid::isValid($id)) {
            $io->error('The cloud library id must be a UUID.');

            return Command::FAILURE;
        }
        $library = $this->entityManager->find(CloudLibrary::class, Uuid::fromString($id));
        if (!$library instanceof CloudLibrary) {
            $io->error(\sprintf('No cloud library with id %s.', $id));

            return Command::FAILURE;
        }

        $start = hrtime(true);
        $this->entityManager->wrapInTransaction(fn () => $this->judge->rebuild($library));
        ['applied' => $applied, 'request' => $requests] = $this->operations->countByStatus($library);
        $io->success(\sprintf('Applied %d operations, %d left as requests, in %.1f s.', $applied, $requests, (hrtime(true) - $start) / 1e9));

        return Command::SUCCESS;
    }
}
