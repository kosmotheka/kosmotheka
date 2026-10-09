<?php

declare(strict_types=1);

namespace App\Tests\Sync\Merge;

use App\Dto\ChangePayload;
use App\Dto\OperationPayload;
use App\Dto\PushedOperation;
use App\Dto\PushRequest;
use App\Dto\PushResult;
use App\Entity\CloudDeviceKey;
use App\Entity\CloudLibrary;
use App\Entity\Device;
use App\Enum\CatalogEntity;
use App\Repository\CloudOperationRepository;
use App\Sync\CloudPush;
use App\Sync\Merge\CatalogMerge;
use App\Sync\OperationSignature;
use App\Sync\Permission\CatalogJudge;
use App\Sync\SchemaVersion;
use App\Tests\Support\CatalogRows;
use App\Tests\Support\CatalogState;
use App\Tests\Support\Judged;
use App\Tests\Support\JudgedTestTrait;
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
    use JudgedTestTrait;

    /** Operations applied between two flushes, as a push or a rebuild would. */
    private const int FLUSH_EVERY = 100;

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
        $this->ingest($batched, 300, self::FLUSH_EVERY);
        $this->ingest($single, 300, 1);

        $state = CatalogState::of($this->em, $batched);
        self::assertEquals($state, CatalogState::of($this->em, $single));
        $this->assertCounts($state, 300);
    }

    /**
     * `make stress BOOKS=… LIMIT=…`. The operations go through the judge, one `take` each and a `settle` every
     * FLUSH_EVERY, as pushes will. STRESS_ROLES=1 gives the library an administrator and an editor who writes
     * half of it, so every operation is judged against roles. STRESS_NO_TRIGGERS=1 switches the same-library
     * triggers off inside the test's own transaction (DAMA rolls it back), only to compare.
     */
    #[Group('stress')]
    public function testALargeLibrary(): void
    {
        $books = $this->stressBooks();
        $library = $this->library();
        $noTriggers = $this->flag('STRESS_NO_TRIGGERS');
        $roles = $this->flag('STRESS_ROLES');
        if ($noTriggers) {
            $this->setTriggers(false);
        }

        $start = hrtime(true);
        [$operations, $blocks] = $this->ingestJudged($library, $books, $roles);
        $rebuild = (hrtime(true) - $start) / 1e9;
        $state = CatalogState::of($this->em, $library);
        $this->assertCounts($state, $books, $roles);
        self::assertSame(0, self::getContainer()->get(CloudOperationRepository::class)->countByStatus($library)['request'], 'every operation of the history is allowed');
        $library = $this->em->find(CloudLibrary::class, $library->getId()) ?? self::fail('The cloud library is gone.');

        if ($this->flag('STRESS_ANALYZE')) {
            // VACUUM cannot run inside the test's transaction; ANALYZE can, and tells whether statistics are what makes pushes slow.
            foreach (['cloud_operation', ...array_map(static fn (CatalogEntity $entity): string => $entity->table(), CatalogEntity::cases())] as $table) {
                $this->em->getConnection()->executeStatement("ANALYZE $table");
            }
        }

        // The pushes go through CloudPush as a device would send them: signed, chained to a key the log knows.
        $uploader = $this->uploader($library);
        $pushes = [];
        for ($n = 0; $n < 30; ++$n) {
            $started = hrtime(true);
            $result = $this->push($library, $uploader, LibraryHistory::push($n, $roles));
            $pushes[] = (hrtime(true) - $started) / 1e6;
            self::assertSame(2, $result->received);
        }
        sort($pushes);

        if ($noTriggers) {
            $this->setTriggers(true);
        }
        fwrite(\STDERR, \sprintf(
            "\nSTRESS books=%d roles=%s triggers=%s: %d operations, %d rows, %d versions · judged %.1f s · first 1000 operations %.1f s, last 1000 %.1f s · push of 2 operations (one transaction): median %.1f ms, worst %.1f ms · peak %d MB\n",
            $books, $roles ? 'yes' : 'no', $noTriggers ? 'off' : 'on', $operations, array_sum(CatalogState::countsPerEntity($state)), \count($state['versions']),
            $rebuild, $blocks[0] ?? 0, [] === $blocks ? 0 : end($blocks), $pushes[14], $pushes[29], memory_get_peak_usage(true) / 1_048_576,
        ));
        self::assertTrue($this->em->isOpen());
    }

    /**
     * `make stress FILTER=testRebuildingALibraryThatHasACatalog`. What `app:catalog:rebuild` does to a library whose
     * catalog is already written: it deletes it and writes it again in the same transaction, so the second
     * writing works in front of the rows the first one left dead.
     */
    #[Group('stress')]
    public function testRebuildingALibraryThatHasACatalog(): void
    {
        $books = $this->stressBooks();
        $library = $this->library();
        $roles = $this->flag('STRESS_ROLES');

        $start = hrtime(true);
        $this->ingestJudged($library, $books, $roles);
        $first = (hrtime(true) - $start) / 1e9;
        $state = CatalogState::of($this->em, $library);

        $judge = self::getContainer()->get(CatalogJudge::class);
        $rebuilds = [];
        for ($n = 0; $n < 3; ++$n) {
            $start = hrtime(true);
            $judge->rebuild($library);
            $rebuilds[] = (hrtime(true) - $start) / 1e9;
        }

        self::assertEquals($state, CatalogState::of($this->em, $library));
        fwrite(\STDERR, \sprintf(
            "\nREBUILD books=%d roles=%s: judged as pushes %.1f s · rebuilt over its own catalog %s s\n",
            $books, $roles ? 'yes' : 'no', $first, implode(', ', array_map(static fn (float $s): string => \sprintf('%.1f', $s), $rebuilds)),
        ));
    }

    /** What one simple push (one field of one work) asks the database, and the most expensive of it, explained. */
    #[Group('stress')]
    public function testWhatOnePushAsks(): void
    {
        $library = $this->library();
        $roles = $this->flag('STRESS_ROLES');
        $this->ingestJudged($library, $this->stressBooks(), $roles);

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
            $push = Ops::operation(Ops::hlc(LibraryHistory::PUSHES_FROM, 0, LibraryHistory::ADMIN_DEVICE), [Ops::update(CatalogEntity::Work, LibraryHistory::id('w', 100), ['title' => 'Otro título'])], signature: Judged::SIGNATURE);
            $this->judged($library)->deliver($push);
        });
        $log = QueryLogStatement::log();
        $native->setAttribute(\PDO::ATTR_STATEMENT_CLASS, [\PDOStatement::class]);

        $out = \sprintf("\nONE PUSH (a title of one work), %d statements:\n", \count($log));
        foreach ($log as $i => $query) {
            $out .= \sprintf("%2d %6.2f ms  %s\n", $i + 1, $query['ms'], preg_replace('/\s+/', ' ', $query['sql']));
        }
        $selects = array_filter($log, static fn (array $q): bool => str_starts_with($q['sql'], 'SELECT') || str_starts_with($q['sql'], 'UPDATE'));
        usort($selects, static fn (array $a, array $b): int => $b['ms'] <=> $a['ms']);
        if ([] !== $selects) {
            $slowest = $selects[0];
            $out .= \sprintf("\nMost expensive SELECT or UPDATE (%.2f ms): %s\n", $slowest['ms'], $slowest['sql']);
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
     * Stores the history in the log (what the pushes did) and judges it operation by operation, settling the
     * library every FLUSH_EVERY. Only the judging is timed.
     *
     * @return array{int, list<float>} operations judged, and the seconds each block of 1000 took
     */
    private function ingestJudged(CloudLibrary $library, int $books, bool $roles): array
    {
        $judged = $this->judged($library);
        $history = iterator_to_array(LibraryHistory::operations($books, $roles), false);
        foreach ($history as $operation) {
            $judged->store($operation);
        }
        $judge = self::getContainer()->get(CatalogJudge::class);

        $applied = 0;
        $blocks = [];
        $mark = hrtime(true);
        foreach ($history as $operation) {
            $judge->take($library, $operation);
            if (0 === ++$applied % self::FLUSH_EVERY) {
                $judge->settle($library);
            }
            if (0 === $applied % 1000) {
                $blocks[] = (hrtime(true) - $mark) / 1e9;
                $mark = hrtime(true);
            }
        }
        $judge->settle($library);

        return [$applied, $blocks];
    }

    /** The device that uploads: one of the library's owner, which also knows the keys of the history's authors. */
    private function uploader(CloudLibrary $library): Device
    {
        $library->link(LibraryHistory::LIBRARY);
        foreach ([LibraryHistory::ADMIN_DEVICE, LibraryHistory::EDITOR_DEVICE] as $author) {
            $this->em->persist(new CloudDeviceKey($library, $author, $this->publicKey()));
        }
        $uploader = new Device(bin2hex(random_bytes(16)), $library->getOwner(), 'Uploader', $this->publicKey());
        $this->em->persist($uploader);
        $this->em->flush();

        return $uploader;
    }

    /**
     * @param list<OperationPayload> $operations
     */
    private function push(CloudLibrary $library, Device $uploader, array $operations): PushResult
    {
        $version = self::getContainer()->get(SchemaVersion::class)->current();
        $secret = sodium_crypto_sign_secretkey(sodium_crypto_sign_seed_keypair(str_repeat("\x01", 32)));
        $entries = [];
        foreach ($operations as $operation) {
            $body = json_decode(json_encode([
                'id' => $operation->id,
                'library' => $operation->library,
                'hlc' => $operation->hlc,
                'intent' => $operation->intent,
                'changes' => array_map(static fn (ChangePayload $change): array => array_filter([
                    'entity' => $change->entity->value,
                    'id' => $change->id,
                    'action' => $change->action->value,
                    'fields' => [] === $change->fields ? null : $change->fields,
                ], static fn (mixed $value): bool => null !== $value), $operation->changes),
                'previous' => $operation->previous,
            ], \JSON_THROW_ON_ERROR), false, flags: \JSON_THROW_ON_ERROR);
            \assert($body instanceof \stdClass);
            $body->signature = sodium_bin2base64(sodium_crypto_sign_detached(new OperationSignature()->canonical($body), $secret), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
            $entries[] = new PushedOperation($version, $body);
        }

        return self::getContainer()->get(CloudPush::class)->push($library, new PushRequest($version, $entries), $uploader);
    }

    private function publicKey(): string
    {
        return sodium_bin2base64(sodium_crypto_sign_publickey(sodium_crypto_sign_seed_keypair(str_repeat("\x01", 32))), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
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
    private function assertCounts(array $state, int $books, bool $roles = false): void
    {
        ['kept' => $kept, 'deleted' => $deleted] = LibraryHistory::expectedBooks($books);

        self::assertSame([
            'book_details' => $kept,
            'copy' => $kept,
            ...($roles ? ['device' => 2] : []),
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
