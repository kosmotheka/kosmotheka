<?php

declare(strict_types=1);

namespace App\Command;

use App\Entity\CloudLibrary;
use App\Enum\ChangeOutcome;
use App\Repository\CloudOperationRepository;
use App\Sync\Merge\CatalogMerge;
use App\Sync\OperationReader;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Style\SymfonyStyle;
use Symfony\Component\Uid\Uuid;

/**
 * Rebuilds the catalog of a cloud library from its operation log (milestone 7.4.4):
 * the log is the record, the catalog is derived from it.
 */
#[AsCommand(name: 'app:catalog:rebuild', description: 'Rebuilds the catalog of a cloud library from its operation log')]
final class CatalogRebuildCommand extends Command
{
    /** Operations applied between two flushes: it bounds the queue of statements. The catalog is the same whatever this is. */
    public const int FLUSH_EVERY = 100;

    /** Operations read from the log at a time. */
    private const int PAGE = 500;

    public function __construct(
        private readonly EntityManagerInterface $entityManager,
        private readonly CatalogMerge $merge,
        private readonly CloudOperationRepository $operations,
        private readonly OperationReader $reader,
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
        [$applied, $skipped] = $this->entityManager->wrapInTransaction(fn (): array => $this->rebuild($id));
        $io->success(\sprintf('Applied %d operations, skipped %d changes, in %.1f s.', $applied, $skipped, (hrtime(true) - $start) / 1e9));

        return Command::SUCCESS;
    }

    /**
     * @return array{int, int} operations applied and changes skipped
     */
    private function rebuild(string $id): array
    {
        $library = $this->library($id);
        $this->merge->clear($library);

        $applied = $skipped = 0;
        $after = '';
        while ([] !== $page = $this->operations->findPage($library, $after, self::PAGE)) {
            foreach ($page as ['hlc' => $hlc, 'operation' => $operation]) {
                $outcomes = $this->merge->apply($library, $this->reader->read($operation));
                $skipped += \count(array_filter($outcomes, static fn (ChangeOutcome $outcome): bool => ChangeOutcome::Skipped === $outcome));
                $after = $hlc;
                if (0 === ++$applied % self::FLUSH_EVERY) {
                    $this->merge->flush();
                }
            }
        }
        $this->merge->flush();

        return [$applied, $skipped];
    }

    private function library(string $id): CloudLibrary
    {
        return $this->entityManager->find(CloudLibrary::class, Uuid::fromString($id)) ?? throw new \LogicException('The cloud library disappeared during the rebuild.');
    }
}
