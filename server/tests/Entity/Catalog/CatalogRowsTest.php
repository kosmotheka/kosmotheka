<?php

declare(strict_types=1);

namespace App\Tests\Entity\Catalog;

use App\Entity\Catalog\BookDetails;
use App\Entity\Catalog\ComicDetails;
use App\Entity\Catalog\Copy;
use App\Entity\Catalog\Edition;
use App\Entity\Catalog\FieldVersion;
use App\Entity\Catalog\Member;
use App\Entity\Catalog\Publisher;
use App\Entity\Catalog\RowAuthor;
use App\Entity\Catalog\Tombstone;
use App\Entity\Catalog\Work;
use App\Tests\Support\CatalogRows;
use Doctrine\DBAL\Connection;
use Doctrine\DBAL\Exception\ForeignKeyConstraintViolationException;
use Doctrine\DBAL\Exception\UniqueConstraintViolationException;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;

final class CatalogRowsTest extends KernelTestCase
{
    private EntityManagerInterface $em;
    private Connection $db;
    private CatalogRows $rows;

    protected function setUp(): void
    {
        self::bootKernel();
        $this->em = self::getContainer()->get(EntityManagerInterface::class);
        $this->db = self::getContainer()->get(Connection::class);
        $this->rows = new CatalogRows($this->em);
    }

    /**
     * @return iterable<string, array{string}>
     */
    public static function crossReferences(): iterable
    {
        yield 'required reference (edition.work)' => ['edition.work'];
        yield 'optional reference (copy.owner)' => ['copy.owner'];
        yield 'details (book_details.edition)' => ['book_details.edition'];
    }

    /**
     * Hostile: with one-column foreign keys the database no longer stops a row of
     * library A pointing into library B by itself; the same-library triggers do.
     */
    #[DataProvider('crossReferences')]
    public function testARowCannotPointIntoAnotherLibrary(string $case): void
    {
        $mine = $this->rows->cloudLibrary();
        $theirs = $this->rows->cloudLibrary();
        $work = $this->rows->row(Work::class, $mine, 'w1', ['kind' => 'book', 'title' => 'Mine']);
        $edition = $this->rows->row(Edition::class, $mine, 'e1', ['work' => $work, 'title' => 'Mine']);
        $foreignWork = $this->rows->row(Work::class, $theirs, 'w2', ['kind' => 'book', 'title' => 'Theirs']);
        $foreignEdition = $this->rows->row(Edition::class, $theirs, 'e2', ['work' => $foreignWork, 'title' => 'Theirs']);
        $foreignMember = $this->rows->row(Member::class, $theirs, 'm2', ['name' => 'Tanaka']);
        $this->em->flush();

        match ($case) {
            'edition.work' => $this->rows->row(Edition::class, $mine, 'e9', ['work' => $foreignWork, 'title' => 'Crossed']),
            'copy.owner' => $this->rows->row(Copy::class, $mine, 'c9', ['edition' => $edition, 'owner' => $foreignMember, 'format' => 'paperback', 'quantity' => 1]),
            default => $this->rows->row(BookDetails::class, $mine, 'e1', ['edition' => $foreignEdition]),
        };

        $this->expectException(ForeignKeyConstraintViolationException::class);
        $this->em->flush();
    }

    public function testTheSameIdInTwoLibrariesAreTwoRows(): void
    {
        $this->rows->row(Work::class, $this->rows->cloudLibrary(), 'w1', ['kind' => 'book', 'title' => 'A']);
        $this->rows->row(Work::class, $this->rows->cloudLibrary(), 'w1', ['kind' => 'book', 'title' => 'B']);
        $this->em->flush();

        self::assertSame(2, $this->em->getRepository(Work::class)->count(['id' => 'w1']));
    }

    public function testTheSameIdTwiceInOneLibraryIsRejected(): void
    {
        $library = $this->rows->cloudLibrary();
        $this->rows->row(Work::class, $library, 'w1', ['kind' => 'book', 'title' => 'A']);
        $this->rows->row(Work::class, $library, 'w1', ['kind' => 'book', 'title' => 'B']);

        $this->expectException(UniqueConstraintViolationException::class);
        $this->em->flush();
    }

    public function testAnEditionWithoutPublisherIsSavedAndFound(): void
    {
        $library = $this->rows->cloudLibrary();
        $work = $this->rows->row(Work::class, $library, 'w1', ['kind' => 'book', 'title' => 'Dune']);
        $this->rows->row(Edition::class, $library, 'e1', ['work' => $work, 'title' => 'Dune']);
        $this->em->flush();

        self::assertSame(1, $this->em->getRepository(Edition::class)->count(['cloudLibrary' => $library, 'publisher' => null]));
    }

    public function testDeletingAPublisherLeavesTheEditionWithoutOne(): void
    {
        $library = $this->rows->cloudLibrary();
        $work = $this->rows->row(Work::class, $library, 'w1', ['kind' => 'book', 'title' => 'Dune']);
        $publisher = $this->rows->row(Publisher::class, $library, 'p1', ['name' => 'Acme']);
        $edition = $this->rows->row(Edition::class, $library, 'e1', ['work' => $work, 'title' => 'Dune', 'publisher' => $publisher]);
        $this->em->flush();

        $this->em->remove($publisher);
        $this->em->flush();
        $this->em->refresh($edition);

        self::assertNull($edition->getPublisher());
    }

    public function testDeletingAWorkTakesItsEditionsCopiesAndDetails(): void
    {
        $library = $this->rows->cloudLibrary();
        $work = $this->rows->row(Work::class, $library, 'w1', ['kind' => 'book', 'title' => 'Dune']);
        $edition = $this->rows->row(Edition::class, $library, 'e1', ['work' => $work, 'title' => 'Dune']);
        $this->rows->row(Copy::class, $library, 'c1', ['edition' => $edition, 'format' => 'paperback', 'quantity' => 1]);
        $this->rows->row(BookDetails::class, $library, 'e1', ['edition' => $edition]);
        $this->em->flush();

        $this->db->executeStatement('DELETE FROM catalog_works WHERE id = :id', ['id' => 'w1']);

        foreach (['catalog_editions', 'catalog_copies', 'catalog_book_details'] as $table) {
            self::assertEquals(0, $this->db->fetchOne(\sprintf('SELECT count(*) FROM %s', $table)), $table);
        }
    }

    public function testAnEditionCanHaveTwoKindsOfDetails(): void
    {
        $library = $this->rows->cloudLibrary();
        $work = $this->rows->row(Work::class, $library, 'w1', ['kind' => 'book', 'title' => 'Dune']);
        $edition = $this->rows->row(Edition::class, $library, 'e1', ['work' => $work, 'title' => 'Dune']);
        $this->rows->row(BookDetails::class, $library, 'e1', ['edition' => $edition]);
        $this->rows->row(ComicDetails::class, $library, 'e1', ['edition' => $edition, 'number' => '3']);
        $this->em->flush();

        self::assertSame(1, $this->em->getRepository(BookDetails::class)->count([]));
        self::assertSame(1, $this->em->getRepository(ComicDetails::class)->count([]));
    }

    public function testDeletingTheCloudLibraryTakesItsRowsAndBookkeepingOnly(): void
    {
        $gone = $this->rows->cloudLibrary();
        $kept = $this->rows->cloudLibrary();
        foreach ([$gone, $kept] as $library) {
            $work = $this->rows->row(Work::class, $library, 'w1', ['kind' => 'book', 'title' => 'Dune']);
            $this->rows->row(Edition::class, $library, 'e1', ['work' => $work, 'title' => 'Dune']);
            $this->em->persist(new FieldVersion($library, 'work', 'w1', 'title', '1'));
            $this->em->persist(new Tombstone($library, 'work', 'w9', '2'));
            $this->em->persist(new RowAuthor($library, 'work', 'w1', 'device'));
        }
        $this->em->flush();

        $this->db->executeStatement('DELETE FROM cloud_library WHERE id = :id', ['id' => $gone->getId()->toRfc4122()]);

        foreach (['catalog_works', 'catalog_editions', 'catalog_field_versions', 'catalog_tombstones', 'catalog_row_authors'] as $table) {
            self::assertSame(
                [$kept->getId()->toRfc4122()],
                $this->db->fetchFirstColumn(\sprintf('SELECT cloud_library_id FROM %s', $table)),
                $table,
            );
        }
    }
}
