<?php

declare(strict_types=1);

namespace App\Tests\Sync;

use App\Enum\CatalogEntity;
use App\Enum\FieldType;
use Doctrine\DBAL\Connection;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;

/**
 * The catalog tables (milestone 7.4.3) are written by hand, so nothing but this test
 * keeps them in step with CatalogEntity (the mirror of crates/domain/src/schema.rs):
 * a field without a column, or a column without a field, fails here.
 */
final class CatalogTablesTest extends KernelTestCase
{
    /** Bookkeeping tables of the merge and the permissions; not entities. */
    private const array BOOKKEEPING = ['catalog_field_versions', 'catalog_tombstones', 'catalog_row_authors'];

    /**
     * The link of a library is the server's own cloud_library.id (D84), so these two
     * fields of the `library` row have no column: the one column with that name is
     * the key of every table.
     */
    private const array NOT_STORED = ['cloud_library_id', 'cloud_server'];

    private Connection $db;

    protected function setUp(): void
    {
        self::bootKernel();
        $this->db = self::getContainer()->get(Connection::class);
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

    #[DataProvider('entities')]
    public function testEveryEntityHasExactlyTheColumnsOfItsFields(CatalogEntity $entity): void
    {
        $expected = ['cloud_library_id' => ['uuid', false], 'id' => ['text', false]];
        foreach ($entity->fields() as $name => $spec) {
            if (CatalogEntity::Library === $entity && \in_array($name, self::NOT_STORED, true)) {
                continue;
            }
            $expected[$name] = [
                match ($spec->type) {
                    FieldType::Integer => 'bigint',
                    FieldType::Boolean => 'boolean',
                    FieldType::Rating => 'integer',
                    FieldType::Count => 'bigint',
                    FieldType::Isbn13 => 'text',
                    default => 'text',
                },
                !$spec->required,
            ];
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

        self::assertNotEmpty($actual, \sprintf('Table %s does not exist.', $entity->table()));
        ksort($expected);
        ksort($actual);
        self::assertSame($expected, $actual, $entity->table());
    }

    #[DataProvider('entities')]
    public function testEveryForeignKeyCarriesTheLibraryAndFollowsTheDeleteRule(CatalogEntity $entity): void
    {
        // The library, always; a required reference cascades and an optional one is
        // nulled, only that column (D89).
        $expected = ['cloud_library_id -> cloud_library(id) cascade'];
        foreach ($entity->fields() as $name => $spec) {
            if (null === $spec->references) {
                continue;
            }
            $expected[] = $spec->required
                ? \sprintf('cloud_library_id,%s -> %s(cloud_library_id,id) cascade', $name, $spec->references->table())
                : \sprintf('cloud_library_id,%s -> %s(cloud_library_id,id) set null (%s)', $name, $spec->references->table(), $name);
        }
        // Every kind's details are the row of their edition.
        if (str_ends_with($entity->value, '_details')) {
            $expected[] = 'cloud_library_id,id -> catalog_editions(cloud_library_id,id) cascade';
        }
        sort($expected);

        $actual = $this->foreignKeys($entity->table());
        sort($actual);

        self::assertSame($expected, $actual, $entity->table());
    }

    public function testEveryCatalogTableIsAnEntityOrBookkeeping(): void
    {
        $expected = [...self::BOOKKEEPING, ...array_map(static fn (CatalogEntity $e): string => $e->table(), CatalogEntity::cases())];
        $actual = $this->db->fetchFirstColumn(
            "SELECT table_name FROM information_schema.tables
              WHERE table_schema = current_schema() AND table_name LIKE 'catalog\\_%'",
        );
        sort($expected);
        sort($actual);

        self::assertSame($expected, $actual);
    }

    public function testBookkeepingIsKeyedByLibraryAndComparesHlcsByBytes(): void
    {
        foreach (self::BOOKKEEPING as $table) {
            self::assertContains('cloud_library_id -> cloud_library(id) cascade', $this->foreignKeys($table), $table);
        }
        foreach (['catalog_field_versions', 'catalog_tombstones'] as $table) {
            $collation = $this->db->fetchOne(
                "SELECT collation_name FROM information_schema.columns
                  WHERE table_schema = current_schema() AND table_name = :t AND column_name = 'hlc'",
                ['t' => $table],
            );
            self::assertSame('C', $collation, $table);
        }
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

        /** @var list<array{cols: string, target: string, target_cols: string, rule: string, set_cols: ?string}> $rows */
        $rows = $this->db->fetchAllAssociative(
            \sprintf(
                "SELECT %s AS cols, c.confrelid::regclass::text AS target, %s AS target_cols,
                        c.confdeltype AS rule,
                        (SELECT string_agg(a.attname, ',') FROM unnest(c.confdelsetcols) k(attnum)
                           JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = k.attnum) AS set_cols
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
                'n' => 'set null'.(null === $row['set_cols'] ? ' (all columns)' : \sprintf(' (%s)', $row['set_cols'])),
                default => 'rule '.$row['rule'],
            },
        ), $rows);
    }
}
