<?php

declare(strict_types=1);

namespace App\Sync\Merge;

/**
 * A statement built by CatalogSql and not yet run: its text has only `?` where a value goes, and the
 * values travel apart, each with the property (hence the column type) it is bound to.
 */
final readonly class SqlStatement
{
    /**
     * @param class-string $class      the entity whose mapping gave the table, the columns and the types
     * @param list<string> $properties one per `?`, in order
     * @param list<mixed>  $values     one per `?`, in order
     */
    public function __construct(
        public string $sql,
        public string $class,
        public array $properties,
        public array $values,
    ) {
    }
}
