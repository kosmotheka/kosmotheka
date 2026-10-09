<?php

declare(strict_types=1);

namespace App\Tests\Support;

/**
 * A PDO statement that remembers what it ran and for how long, to see which queries a push makes.
 * Install it with `$pdo->setAttribute(\PDO::ATTR_STATEMENT_CLASS, [QueryLogStatement::class])`. Statements that
 * go through `PDO::exec` (the savepoints) are not seen.
 */
final class QueryLogStatement extends \PDOStatement
{
    /** @var list<array{sql: string, ms: float, params: array<int|string, mixed>}> */
    private static array $log = [];

    /** @var array<int|string, mixed> */
    private array $bound = [];

    public static function reset(): void
    {
        self::$log = [];
    }

    /**
     * @return list<array{sql: string, ms: float, params: array<int|string, mixed>}>
     */
    public static function log(): array
    {
        return self::$log;
    }

    #[\Override]
    public function bindValue(string|int $param, mixed $value, int $type = \PDO::PARAM_STR): bool
    {
        $this->bound[$param] = $value;

        return parent::bindValue($param, $value, $type);
    }

    /**
     * @param array<int|string, mixed>|null $params
     */
    #[\Override]
    public function execute(?array $params = null): bool
    {
        $start = hrtime(true);
        $done = parent::execute($params);
        self::$log[] = ['sql' => $this->queryString, 'ms' => (hrtime(true) - $start) / 1e6, 'params' => $params ?? $this->bound];

        return $done;
    }
}
