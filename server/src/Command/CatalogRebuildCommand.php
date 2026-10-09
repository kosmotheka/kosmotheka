<?php

declare(strict_types=1);

namespace App\Command;

use App\Entity\CloudLibrary;
use App\Repository\CloudOperationRepository;
use App\Sync\Permission\CatalogJudge;
use Doctrine\DBAL\LockMode;
use Doctrine\ORM\EntityManagerInterface;
use Doctrine\Persistence\ManagerRegistry;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
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
        private readonly ManagerRegistry $registry,
        private readonly CatalogJudge $judge,
        private readonly CloudOperationRepository $operations,
    ) {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this
            ->addArgument('cloud-library', InputArgument::OPTIONAL, 'The id of the cloud library (the server\'s own)')
            ->addOption('all', null, InputOption::VALUE_NONE, 'Rebuild every cloud library, one at a time, each in its own transaction');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $io = new SymfonyStyle($input, $output);
        $id = $input->getArgument('cloud-library');
        if ($input->getOption('all') === (null !== $id)) {
            $io->error('Give the id of one cloud library, or --all.');

            return Command::FAILURE;
        }
        if (null === $id) {
            return $this->rebuildAll($io);
        }
        if (!\is_string($id) || !Uuid::isValid($id)) {
            $io->error('The cloud library id must be a UUID.');

            return Command::FAILURE;
        }
        $library = $this->entityManager->find(CloudLibrary::class, Uuid::fromString($id));
        if (!$library instanceof CloudLibrary) {
            $io->error(\sprintf('No cloud library with id %s.', $id));

            return Command::FAILURE;
        }
        $io->success($this->rebuild($library));

        return Command::SUCCESS;
    }

    /** One library after another: a failure is reported and does not stop the rest, but the command ends in error. */
    private function rebuildAll(SymfonyStyle $io): int
    {
        $ids = $this->entityManager->createQuery(\sprintf('SELECT l.id FROM %s l ORDER BY l.createdAt', CloudLibrary::class))->getSingleColumnResult();
        $failed = 0;
        foreach ($ids as $id) {
            $id = $id instanceof Uuid ? $id : Uuid::fromString(\is_string($id) ? $id : '');
            try {
                $library = $this->entityManager->find(CloudLibrary::class, $id) ?? throw new \RuntimeException('The library is gone.');
                $io->writeln(\sprintf('%s: %s', $id->toRfc4122(), $this->rebuild($library)));
            } catch (\Throwable $e) {
                ++$failed;
                $io->writeln(\sprintf('%s: FAILED, %s', $id->toRfc4122(), $e->getMessage()));
                // A failed transaction closes the manager; the next library needs an open one, and nothing the
                // failed one left queued.
                $this->registry->resetManager();
                $this->judge->reset();
            }
            $this->entityManager->clear();
        }
        if ($failed > 0) {
            $io->error(\sprintf('%d of %d libraries failed.', $failed, \count($ids)));

            return Command::FAILURE;
        }
        $io->success(\sprintf('%d libraries rebuilt.', \count($ids)));

        return Command::SUCCESS;
    }

    private function rebuild(CloudLibrary $library): string
    {
        $start = hrtime(true);
        $this->entityManager->wrapInTransaction(function () use ($library): void {
            // The same lock as a push, which would otherwise be judged against a catalog that is being rewritten.
            $this->entityManager->refresh($library, LockMode::PESSIMISTIC_WRITE);
            $this->judge->rebuild($library);
        });
        ['applied' => $applied, 'request' => $requests] = $this->operations->countByStatus($library);

        return \sprintf('Applied %d operations, %d left as requests, in %.1f s.', $applied, $requests, (hrtime(true) - $start) / 1e9);
    }
}
