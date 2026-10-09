<?php

declare(strict_types=1);

namespace App\Sync\Merge;

use Doctrine\DBAL\Statement;
use Doctrine\ORM\EntityManagerInterface;
use Doctrine\ORM\Mapping\ToOneOwningSideMapping;

/**
 * The only class of the merge engine that holds SQL (D92, revised on 2026-10-09). Every statement is built
 * from the Doctrine metadata of an entity class, so one piece of code serves all the catalog tables.
 *
 * The caller names an entity class and properties (a field, or an association to one row). Table and column
 * names come from the mapping, quoted by the platform, and a property the mapping does not know is refused
 * before anything runs. Values are never part of the SQL text: they are bound parameters, each with the DBAL
 * type of its own column.
 */
final class CatalogSql
{
    /** @var array<class-string, array{table: string, columns: array<string, array{name: string, type: string}>}> */
    private array $tables = [];

    /** @var array<string, array{Statement, list<string>}> prepared statement and bound types, by SQL text */
    private array $prepared = [];

    public function __construct(private readonly EntityManagerInterface $entityManager)
    {
    }

    /**
     * Inserts a row: every column of the class, in a fixed order, null where no value is given.
     *
     * @param class-string         $class
     * @param array<string, mixed> $values by property
     */
    public function insert(string $class, array $values): SqlStatement
    {
        $table = $this->table($class);
        $properties = array_keys($table['columns']);
        $this->known($class, array_keys($values));

        return new SqlStatement(
            \sprintf(
                'INSERT INTO %s (%s) VALUES (%s)',
                $table['table'],
                implode(', ', array_map(static fn (string $property): string => $table['columns'][$property]['name'], $properties)),
                implode(', ', array_fill(0, \count($properties), '?')),
            ),
            $class,
            $properties,
            array_map(static fn (string $property): mixed => $values[$property] ?? null, $properties),
        );
    }

    /**
     * @param class-string         $class
     * @param array<string, mixed> $set   by property
     * @param array<string, mixed> $where by property, all equal
     */
    public function update(string $class, array $set, array $where): SqlStatement
    {
        $table = $this->table($class);
        $this->known($class, [...array_keys($set), ...array_keys($where)]);

        return new SqlStatement(
            \sprintf('UPDATE %s SET %s WHERE %s', $table['table'], implode(', ', $this->conditions($table, $set, '=')), implode(' AND ', $this->conditions($table, $where, '='))),
            $class,
            [...array_keys($set), ...array_keys($where)],
            [...array_values($set), ...array_values($where)],
        );
    }

    /**
     * @param class-string         $class
     * @param array<string, mixed> $where by property, all equal
     */
    public function delete(string $class, array $where): SqlStatement
    {
        $table = $this->table($class);
        $this->known($class, array_keys($where));

        return new SqlStatement(
            \sprintf('DELETE FROM %s WHERE %s', $table['table'], implode(' AND ', $this->conditions($table, $where, '='))),
            $class,
            array_keys($where),
            array_values($where),
        );
    }

    /**
     * The first row that meets the conditions, or null.
     *
     * @param class-string         $class
     * @param list<string>         $select properties to read
     * @param array<string, mixed> $where  by property, all equal
     * @param array<string, mixed> $not    by property, all different
     *
     * @return array<string, mixed>|null by property, as the database returns it
     */
    public function first(string $class, array $select, array $where, array $not = []): ?array
    {
        $table = $this->table($class);
        $this->known($class, [...$select, ...array_keys($where), ...array_keys($not)]);

        $row = $this->read(new SqlStatement(
            \sprintf(
                'SELECT %s FROM %s WHERE %s LIMIT 1',
                implode(', ', array_map(static fn (string $property): string => $table['columns'][$property]['name'], $select)),
                $table['table'],
                implode(' AND ', [...$this->conditions($table, $where, '='), ...$this->conditions($table, $not, '<>')]),
            ),
            $class,
            [...array_keys($where), ...array_keys($not)],
            [...array_values($where), ...array_values($not)],
        ));

        return false === $row ? null : array_combine($select, array_values($row));
    }

    public function execute(SqlStatement $statement): void
    {
        $this->prepare($statement)->executeStatement();
    }

    /**
     * @return array<string, mixed>|false
     */
    private function read(SqlStatement $statement): array|false
    {
        return $this->prepare($statement)->executeQuery()->fetchAssociative();
    }

    private function prepare(SqlStatement $statement): Statement
    {
        if (!isset($this->prepared[$statement->sql])) {
            $columns = $this->table($statement->class)['columns'];
            $this->prepared[$statement->sql] = [
                $this->entityManager->getConnection()->prepare($statement->sql),
                array_map(static fn (string $property): string => $columns[$property]['type'], $statement->properties),
            ];
        }
        [$prepared, $types] = $this->prepared[$statement->sql];
        foreach ($statement->values as $position => $value) {
            $prepared->bindValue($position + 1, $value, $types[$position]);
        }

        return $prepared;
    }

    /**
     * @param array{table: string, columns: array<string, array{name: string, type: string}>} $table
     * @param array<string, mixed>                                                            $by    property => value
     *
     * @return list<string>
     */
    private function conditions(array $table, array $by, string $operator): array
    {
        return array_map(static fn (string $property): string => $table['columns'][$property]['name'].' '.$operator.' ?', array_keys($by));
    }

    /**
     * @param class-string $class
     * @param list<string> $properties
     */
    private function known(string $class, array $properties): void
    {
        $columns = $this->table($class)['columns'];
        foreach ($properties as $property) {
            if (!isset($columns[$property])) {
                throw UnknownPropertyException::of($class, $property);
            }
        }
    }

    /**
     * @param class-string $class
     *
     * @return array{table: string, columns: array<string, array{name: string, type: string}>}
     */
    private function table(string $class): array
    {
        if (isset($this->tables[$class])) {
            return $this->tables[$class];
        }
        $platform = $this->entityManager->getConnection()->getDatabasePlatform();
        $metadata = $this->entityManager->getClassMetadata($class);

        $columns = [];
        foreach ($metadata->getFieldNames() as $field) {
            $columns[$field] = ['name' => $platform->quoteSingleIdentifier($metadata->getColumnName($field)), 'type' => (string) $metadata->getTypeOfField($field)];
        }
        foreach ($metadata->getAssociationNames() as $association) {
            $mapping = $metadata->getAssociationMapping($association);
            if (!$mapping instanceof ToOneOwningSideMapping) {
                continue;
            }
            // The column holds the key of the row it points to, so it takes that column's type.
            $join = $mapping->joinColumns[0];
            $target = $this->entityManager->getClassMetadata($metadata->getAssociationTargetClass($association));
            $columns[$association] = ['name' => $platform->quoteSingleIdentifier($join->name), 'type' => (string) $target->getTypeOfField($target->getFieldForColumn($join->referencedColumnName))];
        }

        return $this->tables[$class] = ['table' => $platform->quoteSingleIdentifier($metadata->getTableName()), 'columns' => $columns];
    }
}
