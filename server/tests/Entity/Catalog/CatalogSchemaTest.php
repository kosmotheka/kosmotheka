<?php

declare(strict_types=1);

namespace App\Tests\Entity\Catalog;

use App\Entity\Catalog\CatalogRow;
use App\Entity\Catalog\Fence;
use App\Entity\Catalog\FenceChain;
use App\Entity\Catalog\RowAuthor;
use App\Entity\Catalog\Tombstone;
use App\Enum\CatalogEntity;
use App\Enum\FieldType;
use Doctrine\DBAL\Connection;
use Doctrine\ORM\EntityManagerInterface;
use Doctrine\ORM\Tools\SchemaValidator;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;

/**
 * The catalog tables are created by a migration that also holds hand-written
 * triggers, so this test keeps the tables, the entities and CatalogEntity (the
 * mirror of crates/domain/src/schema.rs) in step.
 */
final class CatalogSchemaTest extends KernelTestCase
{
    private const array BOOKKEEPING = [
        Tombstone::class => 'catalog_tombstones',
        RowAuthor::class => 'catalog_row_authors',
        Fence::class => 'catalog_device_fences',
        FenceChain::class => 'catalog_device_fence_chains',
    ];

    /** D84: the link of a library is the cloud library itself, so these two have no column. */
    private const array NOT_STORED = ['cloud_library_id', 'cloud_server'];

    private Connection $db;
    private EntityManagerInterface $em;

    protected function setUp(): void
    {
        self::bootKernel();
        $this->db = self::getContainer()->get(Connection::class);
        $this->em = self::getContainer()->get(EntityManagerInterface::class);
    }

    /**
     * @return iterable<string, array{CatalogEntity}>
     */
    public static function entities(): iterable
    {
        foreach (CatalogEntity::cases() as $entity) {
            yield $entity->value => [$entity];
        }
    }

    /**
     * Reference columns of an entity: its Ref fields and, for details, the edition they belong to.
     *
     * @return array<string, array{CatalogEntity, bool}> column => [target, required]
     */
    private function references(CatalogEntity $entity): array
    {
        $references = [];
        foreach ($entity->fields() as $name => $spec) {
            if (null !== $spec->references) {
                // `owner_id` is the column `owner_key`; `trashed_by` (D105), `trashed_by_key`.
                $references[(str_ends_with($name, '_id') ? substr($name, 0, -3) : $name).'_key'] = [$spec->references, $spec->required];
            }
        }
        if (str_ends_with($entity->value, '_details')) {
            $references['edition_key'] = [CatalogEntity::Edition, true];
        }

        return $references;
    }

    #[DataProvider('entities')]
    public function testEveryTableHasExactlyTheColumnsOfItsFields(CatalogEntity $entity): void
    {
        $expected = ['key' => ['uuid', false], 'versions' => ['jsonb', false], 'cloud_library_id' => ['uuid', false], 'id' => ['text', false]];
        foreach ($entity->fields() as $name => $spec) {
            if (CatalogEntity::Library === $entity && \in_array($name, self::NOT_STORED, true)) {
                continue;
            }
            if (FieldType::Ref === $spec->type) {
                continue;
            }
            $expected[$name] = [
                match ($spec->type) {
                    FieldType::Integer, FieldType::Count => 'bigint',
                    FieldType::Boolean => 'boolean',
                    FieldType::Rating => 'integer',
                    default => 'text',
                },
                !$spec->required,
            ];
        }
        foreach ($this->references($entity) as $column => [, $required]) {
            $expected[$column] = ['uuid', !$required];
        }

        $actual = [];
        /** @var list<array{column_name: string, data_type: string, is_nullable: string}> $columns */
        $columns = $this->db->fetchAllAssociative(
            'SELECT column_name, data_type, is_nullable FROM information_schema.columns
              WHERE table_schema = current_schema() AND table_name = :t',
            ['t' => $entity->table()],
        );
        foreach ($columns as $row) {
            $actual[$row['column_name']] = [$row['data_type'], 'YES' === $row['is_nullable']];
        }

        self::assertNotSame([], $actual, \sprintf('Table %s does not exist.', $entity->table()));
        ksort($expected);
        ksort($actual);
        self::assertSame($expected, $actual, $entity->table());
    }

    #[DataProvider('entities')]
    public function testForeignKeysFollowTheDeleteRules(CatalogEntity $entity): void
    {
        $expected = ['cloud_library_id -> cloud_library(id) cascade'];
        foreach ($this->references($entity) as $column => [$target, $required]) {
            $expected[] = \sprintf('%s -> %s(key) %s', $column, $target->table(), $required ? 'cascade' : 'set null');
        }
        sort($expected);

        $actual = $this->foreignKeys($entity->table());
        sort($actual);

        self::assertSame($expected, $actual, $entity->table());
    }

    #[DataProvider('entities')]
    public function testTheAppIdIsUniqueInsideItsLibrary(CatalogEntity $entity): void
    {
        $unique = $this->uniqueIndexes($entity->table());

        self::assertContains('cloud_library_id,id', $unique, $entity->table());
        if (CatalogEntity::WorkContributor === $entity) {
            self::assertContains('work_key,person_key,role', $unique);
        }
    }

    #[DataProvider('entities')]
    public function testEveryEntityHasItsClass(CatalogEntity $entity): void
    {
        $class = $entity->entityClass();

        self::assertTrue(is_subclass_of($class, CatalogRow::class), $class);
        self::assertSame($entity->table(), $this->em->getClassMetadata($class)->getTableName());
    }

    #[DataProvider('entities')]
    public function testEveryReferenceHasItsSameLibraryTrigger(CatalogEntity $entity): void
    {
        $expected = array_map(
            static fn (string $column): string => \sprintf('%s_%s_same_library', $entity->table(), $column),
            array_keys($this->references($entity)),
        );
        /** @var list<string> $actual */
        $actual = $this->db->fetchFirstColumn(
            'SELECT t.tgname FROM pg_trigger t WHERE NOT t.tgisinternal AND t.tgrelid = to_regclass(:t)',
            ['t' => $entity->table()],
        );
        sort($expected);
        sort($actual);

        self::assertSame($expected, $actual, $entity->table());
    }

    public function testEveryTableNamesItsLibraryForeignKeyAfterItself(): void
    {
        $tables = [...array_values(self::BOOKKEEPING), ...array_map(static fn (CatalogEntity $e): string => $e->table(), CatalogEntity::cases())];
        foreach ($tables as $table) {
            $name = \sprintf('fk_%s_cloud_library', $table);
            self::assertSame(
                1,
                $this->db->fetchOne(
                    "SELECT count(*) FROM pg_constraint WHERE contype = 'f' AND conname = :name AND conrelid = to_regclass(:t)",
                    ['name' => $name, 't' => $table],
                ),
                $name,
            );
        }
    }

    public function testThereAreNoOtherCatalogTables(): void
    {
        $expected = [...array_values(self::BOOKKEEPING), ...array_map(static fn (CatalogEntity $e): string => $e->table(), CatalogEntity::cases())];
        $actual = $this->db->fetchFirstColumn(
            "SELECT table_name FROM information_schema.tables
              WHERE table_schema = current_schema() AND table_name LIKE 'catalog\\_%'",
        );
        sort($expected);
        sort($actual);

        self::assertSame($expected, $actual);
    }

    public function testBookkeepingFollowsTheLibraryAndComparesHlcsByBytes(): void
    {
        foreach (self::BOOKKEEPING as $class => $table) {
            self::assertSame($table, $this->em->getClassMetadata($class)->getTableName());
            self::assertContains('cloud_library_id -> cloud_library(id) cascade', $this->foreignKeys($table), $table);
        }
        foreach (['catalog_tombstones', 'catalog_device_fences'] as $table) {
            $column = 'catalog_tombstones' === $table ? 'hlc' : 'until_hlc';
            $collation = $this->db->fetchOne(
                'SELECT collation_name FROM information_schema.columns
                  WHERE table_schema = current_schema() AND table_name = :t AND column_name = :c',
                ['t' => $table, 'c' => $column],
            );
            self::assertSame('C', $collation, $table);
        }
    }

    /** The version of each field lives in its row (the `versions` column), not in a table of its own. */
    public function testThereIsNoTableOfFieldVersions(): void
    {
        self::assertFalse($this->db->createSchemaManager()->tablesExist(['catalog_field_versions']));
    }

    public function testTheMappingIsValidAndTheDatabaseMatchesIt(): void
    {
        // Only the catalog classes' mapping: `doctrine:schema:validate` also flags
        // CloudOperation::$body (stdClass vs array), which is not part of this work.
        $validator = new SchemaValidator($this->em);
        foreach ($this->em->getMetadataFactory()->getAllMetadata() as $metadata) {
            if (str_starts_with($metadata->getName(), 'App\Entity\Catalog\\')) {
                self::assertSame([], $validator->validateClass($metadata), $metadata->getName());
            }
        }

        // The migrations' own table is not mapped; the commands filter it out, so does this.
        $pending = array_values(array_filter(
            $validator->getUpdateSchemaList(),
            static fn (string $sql): bool => !str_contains($sql, 'doctrine_migration_versions'),
        ));
        self::assertSame([], $pending);
    }

    /**
     * @return list<string> "cols -> target(cols) rule", one per foreign key of the table
     */
    private function foreignKeys(string $table): array
    {
        $names = static fn (string $rel, string $keys): string => \sprintf(
            "(SELECT string_agg(a.attname, ',' ORDER BY k.ord)
                FROM unnest(%s) WITH ORDINALITY k(attnum, ord)
                JOIN pg_attribute a ON a.attrelid = %s AND a.attnum = k.attnum)",
            $keys,
            $rel,
        );

        /** @var list<array{cols: string, target: string, target_cols: string, rule: string}> $rows */
        $rows = $this->db->fetchAllAssociative(
            \sprintf(
                "SELECT %s AS cols, c.confrelid::regclass::text AS target, %s AS target_cols, c.confdeltype AS rule
                   FROM pg_constraint c
                  WHERE c.contype = 'f' AND c.conrelid = to_regclass(:t)",
                $names('c.conrelid', 'c.conkey'),
                $names('c.confrelid', 'c.confkey'),
            ),
            ['t' => $table],
        );

        return array_map(static fn (array $row): string => \sprintf(
            '%s -> %s(%s) %s',
            $row['cols'],
            $row['target'],
            $row['target_cols'],
            match ($row['rule']) {
                'c' => 'cascade',
                'n' => 'set null',
                default => 'rule '.$row['rule'],
            },
        ), $rows);
    }

    /**
     * @return list<string> the columns of each unique index (primary key excluded), comma-separated
     */
    private function uniqueIndexes(string $table): array
    {
        /** @var list<string> $rows */
        $rows = $this->db->fetchFirstColumn(
            "SELECT (SELECT string_agg(a.attname, ',' ORDER BY k.ord)
                       FROM unnest(i.indkey) WITH ORDINALITY k(attnum, ord)
                       JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = k.attnum)
               FROM pg_index i
              WHERE i.indisunique AND NOT i.indisprimary AND i.indrelid = to_regclass(:t)",
            ['t' => $table],
        );

        return $rows;
    }
}
