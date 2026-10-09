<?php

declare(strict_types=1);

namespace App\Tests\Sync\Merge;

use App\Entity\Catalog\Fence;
use App\Entity\Catalog\FenceChain;
use App\Entity\Catalog\Seal;
use App\Entity\CloudLibrary;
use App\Enum\CatalogEntity;
use App\Repository\CloudOperationRepository;
use App\Sync\Merge\CatalogMerge;
use App\Sync\OperationReader;
use App\Sync\Permission\CatalogJudge;
use App\Tests\Support\CatalogState;
use Doctrine\DBAL\Connection;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Console\Application;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\Console\Tester\CommandTester;
use Symfony\Component\Uid\Uuid;

/**
 * The scenarios of the fixtures give real logs: rebuilding a library from its log gives the same catalog as
 * applying the operations one by one, and as rebuilding it again.
 */
final class CatalogRebuildCommandTest extends WebTestCase
{
    private KernelBrowser $client;
    private EntityManagerInterface $em;

    protected function setUp(): void
    {
        $this->client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        $this->em = self::getContainer()->get(EntityManagerInterface::class);

        $tester = new CommandTester(new Application($this->client->getKernel())->find('doctrine:fixtures:load'));
        $tester->execute([], ['interactive' => false]);
        $tester->assertCommandIsSuccessful();
    }

    public function testRebuildingGivesTheCatalogOfApplyingTheLogOneByOne(): void
    {
        $ids = self::getContainer()->get(Connection::class)->fetchFirstColumn('SELECT DISTINCT cloud_library_id FROM cloud_operation ORDER BY 1');
        self::assertNotSame([], $ids);

        foreach ($ids as $id) {
            \assert(\is_string($id));
            $this->rebuild($id);
            $first = $this->stateOf($id);
            self::assertNotSame([], $first['rows'], $id);
            $statuses = $this->statusesOf($id);
            // The fixtures have roles, so the log holds operations that were allowed and operations that became requests.
            self::assertContains('applied', $statuses, $id);

            $this->oneByOne($id);
            self::assertEquals($first, $this->stateOf($id), 'one by one: '.$id);
            self::assertSame($statuses, $this->statusesOf($id), 'one by one, statuses: '.$id);

            $this->rebuild($id);
            self::assertEquals($first, $this->stateOf($id), 'again: '.$id);
            self::assertSame($statuses, $this->statusesOf($id), 'again, statuses: '.$id);
        }
    }

    public function testTheFixturesLeaveACatalogAndAReadersBookAsARequestOutOfIt(): void
    {
        self::getContainer()->get(Connection::class);

        // Loaded through the push, so the catalog is there without rebuilding anything.
        self::assertSame(0, $this->scalarCount('SELECT COUNT(*) FROM cloud_operation WHERE status IS NULL'));
        self::assertGreaterThan(0, $this->scalarCount("SELECT COUNT(*) FROM catalog_works WHERE title = 'Cien años de soledad'"));
        self::assertSame(0, $this->scalarCount("SELECT COUNT(*) FROM catalog_works WHERE title = 'Ficciones'"));
        self::assertGreaterThanOrEqual(1, $this->scalarCount("SELECT COUNT(*) FROM cloud_operation WHERE status = 'request' AND body ->> 'intent' = 'book.add' AND CAST(body AS TEXT) LIKE '%Ficciones%'"));
    }

    public function testItRefusesWhatIsNotACloudLibrary(): void
    {
        $command = new Application($this->client->getKernel())->find('app:catalog:rebuild');

        foreach (['not-a-uuid', Uuid::v7()->toRfc4122()] as $argument) {
            $tester = new CommandTester($command);
            $tester->execute(['cloud-library' => $argument]);
            self::assertSame(1, $tester->getStatusCode(), $argument);
        }
    }

    public function testAllRebuildsEveryLibraryThatHasALogAndNoCatalog(): void
    {
        $libraries = $this->libraryIds();
        $withLog = array_keys($this->libraryIdsWith('TRUE'));
        self::assertGreaterThanOrEqual(3, \count($withLog));
        $this->forgetTheCatalogs();
        self::assertSame($this->libraryIdsWith('TRUE'), $this->libraryIdsWith('status IS NULL'));
        self::assertSame(0, $this->catalogRows());

        $tester = new CommandTester(new Application($this->client->getKernel())->find('app:catalog:rebuild'));
        $tester->execute(['--all' => true]);

        $tester->assertCommandIsSuccessful();
        foreach ($libraries as $id) {
            self::assertStringContainsString($id.': Applied', $tester->getDisplay());
        }
        self::assertSame([], $this->libraryIdsWith('status IS NULL'));
        self::assertGreaterThan(0, $this->catalogRows());
        $this->em->clear();
        foreach ($withLog as $id) {
            self::assertNotSame([], $this->stateOf($id)['rows'], $id);
        }
    }

    public function testAllGoesOnPastALibraryThatFailsAndEndsInError(): void
    {
        $libraries = $this->libraryIds();
        $this->forgetTheCatalogs();
        // An operation nobody could read: this library cannot be rebuilt.
        $broken = $libraries[0];
        self::getContainer()->get(Connection::class)->executeStatement("UPDATE cloud_operation SET body = '{\"id\": 1}'::jsonb WHERE cloud_library_id = ? AND operation_id = (SELECT operation_id FROM cloud_operation WHERE cloud_library_id = ? ORDER BY hlc LIMIT 1)", [$broken, $broken]);

        $tester = new CommandTester(new Application($this->client->getKernel())->find('app:catalog:rebuild'));
        $tester->execute(['--all' => true]);

        self::assertSame(1, $tester->getStatusCode());
        self::assertStringContainsString($broken.': FAILED', $tester->getDisplay());
        foreach (\array_slice($libraries, 1) as $id) {
            self::assertStringContainsString($id.': Applied', $tester->getDisplay());
        }
    }

    public function testItNeedsOneLibraryOrAll(): void
    {
        $command = new Application($this->client->getKernel())->find('app:catalog:rebuild');

        foreach ([[], ['--all' => true, 'cloud-library' => Uuid::v7()->toRfc4122()]] as $arguments) {
            $tester = new CommandTester($command);
            $tester->execute($arguments);
            self::assertSame(1, $tester->getStatusCode());
        }
    }

    /**
     * @return list<string>
     */
    private function libraryIds(): array
    {
        return array_map(static fn (mixed $id): string => \is_string($id) ? $id : '', self::getContainer()->get(Connection::class)->fetchFirstColumn('SELECT id FROM cloud_library ORDER BY created_at'));
    }

    /**
     * @return array<string, true> the libraries with at least one operation that meets the condition
     */
    private function libraryIdsWith(string $condition): array
    {
        return array_fill_keys(array_map(static fn (mixed $id): string => \is_string($id) ? $id : '', self::getContainer()->get(Connection::class)->fetchFirstColumn("SELECT DISTINCT cloud_library_id FROM cloud_operation WHERE $condition ORDER BY 1")), true);
    }

    /** As the logs of a server that ran before the push judged: operations stored, nothing judged, no catalog. */
    private function forgetTheCatalogs(): void
    {
        $connection = self::getContainer()->get(Connection::class);
        $connection->executeStatement('UPDATE cloud_operation SET status = NULL');
        foreach ([...array_map(static fn (CatalogEntity $entity): string => $entity->table(), CatalogEntity::cases()), 'catalog_tombstones', 'catalog_row_authors', 'catalog_device_fences', 'catalog_device_fence_chains', 'catalog_device_seals'] as $table) {
            $connection->executeStatement("DELETE FROM $table");
        }
        $this->em->clear();
    }

    private function scalarCount(string $sql): int
    {
        $count = self::getContainer()->get(Connection::class)->fetchOne($sql);
        \assert(\is_int($count));

        return $count;
    }

    private function catalogRows(): int
    {
        return $this->scalarCount('SELECT COUNT(*) FROM catalog_works') + $this->scalarCount('SELECT COUNT(*) FROM catalog_members');
    }

    private function rebuild(string $id): void
    {
        $tester = new CommandTester(new Application($this->client->getKernel())->find('app:catalog:rebuild'));
        $tester->execute(['cloud-library' => $id]);
        $tester->assertCommandIsSuccessful();
        self::assertMatchesRegularExpression('/Applied \d+ operations, \d+ left as requests/', $tester->getDisplay());
        $this->em->clear();
    }

    /** Judges the log operation by operation, as pushes would, instead of replaying it in one go. */
    private function oneByOne(string $id): void
    {
        $merge = self::getContainer()->get(CatalogMerge::class);
        $judge = self::getContainer()->get(CatalogJudge::class);
        $operations = self::getContainer()->get(CloudOperationRepository::class);
        $reader = self::getContainer()->get(OperationReader::class);
        $library = $this->library($id);

        $merge->clear($library);
        foreach ([Fence::class, FenceChain::class, Seal::class] as $class) {
            $this->em->createQuery(\sprintf('DELETE FROM %s x WHERE x.cloudLibrary = :library', $class))->setParameter('library', $library->getId(), 'uuid')->execute();
        }
        $after = '';
        while ([] !== $page = $operations->findPage($library, $after, 7)) {
            foreach ($page as ['hlc' => $after, 'operation' => $operation]) {
                $judge->take($library, $reader->read($operation));
            }
        }
        $this->em->clear();
    }

    /**
     * @return array<string, string> status of each operation of the library, by id
     */
    private function statusesOf(string $id): array
    {
        $statuses = [];
        foreach (self::getContainer()->get(Connection::class)->fetchAllKeyValue('SELECT operation_id, status FROM cloud_operation WHERE cloud_library_id = :id', ['id' => $id]) as $operation => $status) {
            $statuses[(string) $operation] = \is_string($status) ? $status : 'none';
        }
        ksort($statuses);

        return $statuses;
    }

    /**
     * @return array{rows: array<string, array<string, mixed>>, versions: array<string, string>, tombstones: array<string, string>}
     */
    private function stateOf(string $id): array
    {
        $state = CatalogState::of($this->em, $this->library($id));
        $this->em->clear();

        return $state;
    }

    private function library(string $id): CloudLibrary
    {
        return $this->em->find(CloudLibrary::class, Uuid::fromString($id)) ?? self::fail('No cloud library '.$id);
    }
}
