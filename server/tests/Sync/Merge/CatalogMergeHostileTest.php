<?php

declare(strict_types=1);

namespace App\Tests\Sync\Merge;

use App\Entity\Catalog\Edition;
use App\Entity\Catalog\Member;
use App\Entity\Catalog\Work;
use App\Entity\Catalog\WorkContributor;
use App\Enum\CatalogEntity;
use App\Enum\ChangeOutcome;
use App\Sync\Merge\CatalogMerge;
use App\Sync\OperationReader;
use App\Sync\OperationRejectedException;
use App\Tests\Support\CatalogRows;
use App\Tests\Support\Ops;
use Doctrine\DBAL\Connection;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;

/**
 * What a modified app can put in an operation. The merge must skip it, with the
 * EntityManager still open, and never let the client reach a class, a property, a
 * table or another library.
 */
final class CatalogMergeHostileTest extends KernelTestCase
{
    private EntityManagerInterface $em;
    private CatalogMerge $merge;
    private CatalogRows $rows;

    protected function setUp(): void
    {
        self::bootKernel();
        $this->em = self::getContainer()->get(EntityManagerInterface::class);
        $this->merge = self::getContainer()->get(CatalogMerge::class);
        $this->rows = new CatalogRows($this->em);
    }

    /**
     * @return iterable<string, array{string}>
     */
    public static function strangeFields(): iterable
    {
        yield 'SQL in the name' => ['work"; DROP TABLE catalog_works; --'];
        yield 'the key of the row' => ['key'];
        yield 'the cloud library column' => ['cloud_library_id'];
        yield 'the id of the app' => ['id'];
        yield 'the property of the library' => ['cloudLibrary'];
        yield 'a property by its camel name' => ['relatedWork'];
        yield 'a numeric name' => ['0'];
    }

    #[DataProvider('strangeFields')]
    public function testAFieldThatIsNotInTheListSkipsTheChange(string $field): void
    {
        $library = $this->rows->cloudLibrary();
        $this->em->flush();

        $outcome = $this->merge->apply($library, Ops::operation(Ops::hlc(1), [
            Ops::create(CatalogEntity::Work, 'w1', ['kind' => 'book', 'title' => 'Dune', $field => 'x']),
        ]));
        $this->merge->flush();

        self::assertSame([ChangeOutcome::Skipped], $outcome);
        self::assertTrue($this->em->isOpen());
        self::assertSame(0, $this->em->getRepository(Work::class)->count([]));
        self::assertSame(0, self::getContainer()->get(Connection::class)->fetchOne('SELECT count(*) FROM catalog_field_versions'));
        // The tables are still there.
        self::assertSame(0, $this->em->getRepository(Work::class)->count([]));
    }

    public function testAnEntityThatIsNotInTheListNeverReachesTheMerge(): void
    {
        $reader = self::getContainer()->get(OperationReader::class);
        $operation = json_decode((string) json_encode([
            'id' => '01997c7e-0000-7000-8000-000000000001',
            'library' => 'l',
            'hlc' => Ops::hlc(1),
            'intent' => 'test',
            'previous' => null,
            'changes' => [['entity' => 'work"; DROP TABLE catalog_works; --', 'id' => 'w1', 'action' => 'create', 'fields' => ['kind' => 'book', 'title' => 'x']]],
        ]), false);
        \assert($operation instanceof \stdClass);

        $this->expectException(OperationRejectedException::class);
        $reader->read($operation);
    }

    public function testAReferenceToARowOfAnotherLibraryIsABrokenReference(): void
    {
        $mine = $this->rows->cloudLibrary();
        $theirs = $this->rows->cloudLibrary();
        $this->rows->row(Work::class, $theirs, 'w-theirs', ['kind' => 'book', 'title' => 'Theirs']);
        $this->em->flush();

        $outcome = $this->merge->apply($mine, Ops::operation(Ops::hlc(1), [
            Ops::create(CatalogEntity::Edition, 'e1', ['work_id' => 'w-theirs', 'title' => 'Crossed']),
        ]));
        $this->merge->flush();

        self::assertSame([ChangeOutcome::Skipped], $outcome);
        self::assertTrue($this->em->isOpen());
        self::assertSame(0, $this->em->getRepository(Edition::class)->count([]));
    }

    public function testTheSameIdInTwoLibrariesIsTwoRowsAndAChangeInOneLeavesTheOtherAlone(): void
    {
        $a = $this->rows->cloudLibrary();
        $b = $this->rows->cloudLibrary();
        // B's rows go in first: a lookup by id alone would find those.
        $this->rows->row(Work::class, $b, 'w1', ['kind' => 'book', 'title' => 'In B']);
        $this->rows->row(Member::class, $b, 'm1', ['name' => 'Bea']);
        $this->em->flush();
        $this->rows->row(Work::class, $a, 'w1', ['kind' => 'book', 'title' => 'In A']);
        $this->em->flush();
        $this->em->clear();
        $a = $this->em->find($a::class, $a->getId()) ?? self::fail();
        $b = $this->em->find($b::class, $b->getId()) ?? self::fail();

        $outcome = $this->merge->apply($a, Ops::operation(Ops::hlc(1), [
            Ops::update(CatalogEntity::Work, 'w1', ['title' => 'Changed in A']),
            Ops::delete(CatalogEntity::Member, 'm1'),
        ]));
        $this->merge->flush();

        // The update lands in A; the delete finds no member in A, but leaves its tombstone there, not in B.
        self::assertSame([ChangeOutcome::Written, ChangeOutcome::Written], $outcome);
        self::assertTrue($this->em->isOpen());
        $titles = [];
        foreach ($this->em->getRepository(Work::class)->findAll() as $work) {
            $titles[$work->getCloudLibrary()->getId()->toRfc4122()] = $work->getTitle();
        }
        self::assertEquals([
            $a->getId()->toRfc4122() => 'Changed in A',
            $b->getId()->toRfc4122() => 'In B',
        ], $titles);
        self::assertSame(1, $this->em->getRepository(Member::class)->count(['cloudLibrary' => $b]));
    }

    public function testAContributorRepeatedInAnotherRowIsSkippedWithoutClosingTheManager(): void
    {
        $library = $this->rows->cloudLibrary();
        $this->em->flush();
        $merge = fn (string $id, int $time): array => $this->merge->apply($library, Ops::operation(Ops::hlc($time), [
            Ops::create(CatalogEntity::WorkContributor, $id, ['work_id' => 'w1', 'person_id' => 'p1', 'role' => 'author', 'position' => 1]),
        ]));
        $this->merge->apply($library, Ops::operation(Ops::hlc(1), [
            Ops::create(CatalogEntity::Work, 'w1', ['kind' => 'book', 'title' => 'Dune']),
            Ops::create(CatalogEntity::Person, 'p1', ['name' => 'Herbert']),
        ]));

        self::assertSame([ChangeOutcome::Written], $merge('c1', 2));
        self::assertSame([ChangeOutcome::Skipped], $merge('c2', 3));
        $this->merge->flush();

        self::assertTrue($this->em->isOpen());
        self::assertSame(1, $this->em->getRepository(WorkContributor::class)->count([]));
    }
}
