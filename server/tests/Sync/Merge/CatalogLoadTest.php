<?php

declare(strict_types=1);

namespace App\Tests\Sync\Merge;

use App\Command\CatalogRebuildCommand;
use App\Entity\CloudLibrary;
use App\Enum\CatalogEntity;
use App\Sync\Merge\CatalogMerge;
use App\Tests\Support\CatalogRows;
use App\Tests\Support\CatalogState;
use App\Tests\Support\LibraryHistory;
use App\Tests\Support\Ops;
use App\Tests\Support\QueryLogStatement;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\Group;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;

/**
 * A made-up library, applied in batches like a rebuild and one operation at a time like pushes: the
 * catalog must be the same. The big one (`make stress`) also times it (D92: it decides whether the engine is fast enough).
 */
final class CatalogLoadTest extends KernelTestCase
{
    private EntityManagerInterface $em;
    private CatalogMerge $merge;

    protected function setUp(): void
    {
        self::bootKernel();
        $this->em = self::getContainer()->get(EntityManagerInterface::class);
        $this->merge = self::getContainer()->get(CatalogMerge::class);
    }

    public function testBatchesAndOneByOneGiveTheSameCatalog(): void
    {
        $batched = $this->library();
        $single = $this->library();
        $this->ingest($batched, 300, CatalogRebuildCommand::FLUSH_EVERY);
        $this->ingest($single, 300, 1);

        $state = CatalogState::of($this->em, $batched);
        self::assertEquals($state, CatalogState::of($this->em, $single));
        $this->assertCounts($state, 300);
    }

    /**
     * `make stress BOOKS=… LIMIT=…`. STRESS_NO_TRIGGERS=1 switches the same-library triggers off inside the
     * test's own transaction (DAMA rolls it back), only to compare.
     */
    #[Group('stress')]
    public function testALargeLibrary(): void
    {
        $books = $this->stressBooks();
        $library = $this->library();
        $noTriggers = $this->flag('STRESS_NO_TRIGGERS');
        if ($noTriggers) {
            $this->setTriggers(false);
        }

        $start = hrtime(true);
        [$operations, $blocks] = $this->ingestTimed($library, $books);
        $rebuild = (hrtime(true) - $start) / 1e9;
        $state = CatalogState::of($this->em, $library);
        $this->assertCounts($state, $books);
        $library = $this->em->find(CloudLibrary::class, $library->getId()) ?? self::fail('The cloud library is gone.');

        $pushes = [];
        for ($n = 0; $n < 30; ++$n) {
            $started = hrtime(true);
            $this->em->wrapInTransaction(function () use ($library, $n): void {
                foreach (LibraryHistory::push($n) as $operation) {
                    $this->merge->apply($library, $operation);
                }
                $this->merge->flush();
            });
            $pushes[] = (hrtime(true) - $started) / 1e6;
        }
        sort($pushes);

        if ($noTriggers) {
            $this->setTriggers(true);
        }
        fwrite(\STDERR, \sprintf(
            "\nSTRESS books=%d triggers=%s: %d operations, %d rows, %d versions · rebuild %.1f s · first 1000 operations %.1f s, last 1000 %.1f s · push of 2 operations (one transaction): median %.1f ms, worst %.1f ms · peak %d MB\n",
            $books, $noTriggers ? 'off' : 'on', $operations, array_sum(CatalogState::countsPerEntity($state)), \count($state['versions']),
            $rebuild, $blocks[0] ?? 0, [] === $blocks ? 0 : end($blocks), $pushes[14], $pushes[29], memory_get_peak_usage(true) / 1_048_576,
        ));
        self::assertTrue($this->em->isOpen());
    }

    /** What one simple push (one field of one work) asks the database, and the most expensive of it, explained. */
    #[Group('stress')]
    public function testWhatOnePushAsks(): void
    {
        $library = $this->library();
        $this->ingestTimed($library, $this->stressBooks());

        // A new kernel, so the merge prepares its statements again and the log sees them (DAMA keeps the same connection).
        self::ensureKernelShutdown();
        self::bootKernel();
        $this->em = self::getContainer()->get(EntityManagerInterface::class);
        $this->merge = self::getContainer()->get(CatalogMerge::class);
        $library = $this->em->find(CloudLibrary::class, $library->getId()) ?? self::fail('The cloud library is gone.');
        $connection = $this->em->getConnection();
        $native = $connection->getNativeConnection();
        \assert($native instanceof \PDO);
        $native->setAttribute(\PDO::ATTR_STATEMENT_CLASS, [QueryLogStatement::class]);
        QueryLogStatement::reset();
        $this->em->wrapInTransaction(function () use ($library): void {
            $this->merge->apply($library, Ops::operation(Ops::hlc(LibraryHistory::PUSHES_FROM), [Ops::update(CatalogEntity::Work, 'w100', ['title' => 'Otro título'])]));
            $this->merge->flush();
        });
        $log = QueryLogStatement::log();
        $native->setAttribute(\PDO::ATTR_STATEMENT_CLASS, [\PDOStatement::class]);

        $out = \sprintf("\nONE PUSH (a title of one work), %d statements:\n", \count($log));
        foreach ($log as $i => $query) {
            $out .= \sprintf("%2d %6.2f ms  %s\n", $i + 1, $query['ms'], preg_replace('/\s+/', ' ', $query['sql']));
        }
        $selects = array_filter($log, static fn (array $q): bool => str_starts_with($q['sql'], 'SELECT'));
        usort($selects, static fn (array $a, array $b): int => $b['ms'] <=> $a['ms']);
        if ([] !== $selects) {
            $slowest = $selects[0];
            $out .= \sprintf("\nMost expensive SELECT (%.2f ms): %s\n", $slowest['ms'], $slowest['sql']);
            $out .= "EXPLAIN (ANALYZE, BUFFERS) with its own parameters:\n";
            $values = array_values($slowest['params']);
            $explain = preg_replace_callback('/\?/', static function () use (&$values, $connection): string {
                $value = array_shift($values);

                return \is_scalar($value) ? $connection->quote((string) $value) : 'NULL';
            }, $slowest['sql']) ?? '';
            $out .= implode("\n", array_map(static fn (mixed $line): string => \is_string($line) ? $line : '', $connection->fetchFirstColumn('EXPLAIN (ANALYZE, BUFFERS) '.$explain)))."\n";
        }
        fwrite(\STDERR, $out);
        self::assertTrue($this->em->isOpen());
    }

    private function library(): CloudLibrary
    {
        $library = new CatalogRows($this->em)->cloudLibrary();
        $this->em->flush();

        return $library;
    }

    /**
     * Applies the history as the rebuild command does: a flush every FLUSH_EVERY operations.
     *
     * @return array{int, list<float>} operations applied, and the seconds each block of 1000 took
     */
    private function ingestTimed(CloudLibrary $library, int $books): array
    {
        $applied = 0;
        $blocks = [];
        $mark = hrtime(true);
        foreach (LibraryHistory::operations($books) as $operation) {
            $this->merge->apply($library, $operation);
            if (0 === ++$applied % CatalogRebuildCommand::FLUSH_EVERY) {
                $this->merge->flush();
            }
            if (0 === $applied % 1000) {
                $blocks[] = (hrtime(true) - $mark) / 1e9;
                $mark = hrtime(true);
            }
        }
        $this->merge->flush();

        return [$applied, $blocks];
    }

    private function setTriggers(bool $on): void
    {
        $tables = [...array_map(static fn (CatalogEntity $entity): string => $entity->table(), CatalogEntity::cases()), 'catalog_tombstones', 'catalog_row_authors'];
        foreach ($tables as $table) {
            $this->em->getConnection()->executeStatement(\sprintf('ALTER TABLE %s %s TRIGGER USER', $table, $on ? 'ENABLE' : 'DISABLE'));
        }
    }

    private function stressBooks(): int
    {
        return (int) (getenv('STRESS_BOOKS') ?: 20_000);
    }

    private function flag(string $name): bool
    {
        return '' !== (string) getenv($name);
    }

    /** @return int operations applied */
    private function ingest(CloudLibrary $library, int $books, int $flushEvery): int
    {
        $applied = 0;
        foreach (LibraryHistory::operations($books) as $operation) {
            $this->merge->apply($library, $operation);
            if (0 === ++$applied % $flushEvery) {
                $this->merge->flush();
            }
        }
        $this->merge->flush();
        self::assertTrue($this->em->isOpen());

        return $applied;
    }

    /**
     * @param array{rows: array<string, array<string, mixed>>, versions: array<string, string>, tombstones: array<string, string>} $state
     */
    private function assertCounts(array $state, int $books): void
    {
        ['kept' => $kept, 'deleted' => $deleted] = LibraryHistory::expectedBooks($books);

        self::assertSame([
            'book_details' => $kept,
            'copy' => $kept,
            'edition' => $kept,
            'member' => LibraryHistory::MEMBERS,
            'person' => LibraryHistory::PEOPLE,
            'publisher' => LibraryHistory::PUBLISHERS,
            'work' => $kept,
            'work_contributor' => $kept,
        ], CatalogState::countsPerEntity($state));
        self::assertCount($deleted, $state['tombstones']);
    }
}
