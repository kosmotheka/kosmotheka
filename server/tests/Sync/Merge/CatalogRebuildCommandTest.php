<?php

declare(strict_types=1);

namespace App\Tests\Sync\Merge;

use App\Entity\Catalog\Fence;
use App\Entity\Catalog\FenceChain;
use App\Entity\CloudLibrary;
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

    public function testItRefusesWhatIsNotACloudLibrary(): void
    {
        $command = new Application($this->client->getKernel())->find('app:catalog:rebuild');

        foreach (['not-a-uuid', Uuid::v7()->toRfc4122()] as $argument) {
            $tester = new CommandTester($command);
            $tester->execute(['cloud-library' => $argument]);
            self::assertSame(1, $tester->getStatusCode(), $argument);
        }
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
        foreach ([Fence::class, FenceChain::class] as $class) {
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
