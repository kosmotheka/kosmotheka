<?php

declare(strict_types=1);

namespace App\Tests\Sync\Merge;

use App\Entity\Catalog\Work;
use App\Sync\Merge\CatalogSql;
use App\Sync\Merge\SqlStatement;
use App\Sync\Merge\UnknownPropertyException;
use App\Tests\Support\CatalogRows;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;

/**
 * The one class of the merge that holds SQL. What it is asked for may come, through a bug, from client text:
 * a property that is not in the mapping must stop it before anything runs, and a value must never be part of the SQL.
 */
final class CatalogSqlTest extends KernelTestCase
{
    private EntityManagerInterface $em;
    private CatalogSql $sql;

    protected function setUp(): void
    {
        self::bootKernel();
        $this->em = self::getContainer()->get(EntityManagerInterface::class);
        $this->sql = self::getContainer()->get(CatalogSql::class);
    }

    /**
     * @return iterable<string, array{string}>
     */
    public static function propertiesOutOfTheMapping(): iterable
    {
        yield 'a name that is not there' => ['nothing'];
        yield 'a column, not a property' => ['cloud_library_id'];
        yield 'sql in a column list' => ['title) VALUES (1); DROP TABLE catalog_works; --'];
        yield 'sql in a condition' => ["id = 'x' OR 1 = 1; --"];
        yield 'empty' => [''];
    }

    #[DataProvider('propertiesOutOfTheMapping')]
    public function testAPropertyThatIsNotMappedStopsEveryKindOfStatementAndRunsNothing(string $property): void
    {
        $library = new CatalogRows($this->em)->cloudLibrary();
        $work = new CatalogRows($this->em)->row(Work::class, $library, 'w1', ['kind' => 'book', 'title' => 'Dune']);
        $this->em->flush();

        $asks = [
            'insert' => fn (): SqlStatement => $this->sql->insert(Work::class, [$property => 'x']),
            'update, set' => fn (): SqlStatement => $this->sql->update(Work::class, [$property => 'x'], ['id' => 'w1']),
            'update, where' => fn (): SqlStatement => $this->sql->update(Work::class, ['title' => 'x'], [$property => 'w1']),
            'delete' => fn (): SqlStatement => $this->sql->delete(Work::class, [$property => 'w1']),
            'first, select' => fn (): ?array => $this->sql->first(Work::class, [$property], ['id' => 'w1']),
            'first, where' => fn (): ?array => $this->sql->first(Work::class, ['id'], [$property => 'w1']),
            'first, not' => fn (): ?array => $this->sql->first(Work::class, ['id'], ['id' => 'w1'], [$property => 'w1']),
        ];
        foreach ($asks as $what => $ask) {
            try {
                $ask();
                self::fail($what.': it did not refuse the property.');
            } catch (UnknownPropertyException $e) {
                self::assertStringContainsString(Work::class, $e->getMessage(), $what);
            }
        }

        $this->em->clear();
        $kept = $this->em->getRepository(Work::class)->findOneBy(['id' => 'w1']) ?? self::fail('The work is gone.');
        self::assertSame('Dune', $kept->getTitle());
        self::assertSame($work->getKey()->toRfc4122(), $kept->getKey()->toRfc4122());
    }

    public function testValuesNeverReachTheSqlText(): void
    {
        $hostile = "x'); DROP TABLE catalog_works; --";

        $insert = $this->sql->insert(Work::class, ['id' => $hostile, 'title' => $hostile]);
        $update = $this->sql->update(Work::class, ['title' => $hostile], ['id' => $hostile]);
        $delete = $this->sql->delete(Work::class, ['id' => $hostile]);

        foreach ([$insert, $update, $delete] as $statement) {
            self::assertStringNotContainsString('DROP', $statement->sql);
            self::assertCount(substr_count($statement->sql, '?'), $statement->values);
            self::assertCount(\count($statement->properties), $statement->values);
        }
        self::assertContains($hostile, $insert->values);
        // Names come quoted from the mapping.
        self::assertStringContainsString('"catalog_works"', $insert->sql);
        self::assertStringContainsString('"key"', $insert->sql);
    }
}
