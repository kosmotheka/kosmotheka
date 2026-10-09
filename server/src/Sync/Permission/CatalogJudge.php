<?php

declare(strict_types=1);

namespace App\Sync\Permission;

use App\Dto\OperationPayload;
use App\Entity\Catalog\Fence;
use App\Entity\Catalog\FenceChain;
use App\Entity\CloudLibrary;
use App\Enum\CatalogEntity;
use App\Enum\ChangeAction;
use App\Enum\OperationStatus;
use App\Repository\CloudOperationRepository;
use App\Sync\Merge\CatalogMerge;
use App\Sync\OperationReader;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Uid\Uuid;

/**
 * Judges the operations of a cloud library against the permission rules, applies those that are allowed
 * and leaves the others as requests (port of the judging part of `sync::merge_in` and of
 * `permissions::rebuild` in app/src-tauri). It opens no transaction: the caller does.
 *
 * Before it asks anything of the database the merge has nothing queued: every `apply` is followed by a
 * `flush`, so the view reads what the merge wrote.
 *
 * @phpstan-type FenceCandidate array{operation: string, device: string, through: string, until: string}
 */
final class CatalogJudge
{
    /** Operations read from the log at a time. */
    private const int PAGE = 500;

    /** @var array<string, ?string> by cloud library: the newest HLC the library had before this batch */
    private array $newest = [];

    /** @var array<string, true> cloud libraries something arrived late in */
    private array $late = [];

    public function __construct(
        private readonly EntityManagerInterface $entityManager,
        private readonly CatalogMerge $merge,
        private readonly CloudOperationRepository $operations,
        private readonly OperationReader $reader,
    ) {
    }

    /**
     * Judges an operation already stored in the log, applies it or leaves it as a request, and records which.
     * A fence it sets, if it is applied, is noted, and if that puts under suspicion an operation the log
     * already held, the library arrived late too.
     */
    public function take(CloudLibrary $library, OperationPayload $operation): OperationStatus
    {
        $id = $library->getId()->toRfc4122();
        // The newest operation the library had before this batch: one older than that may change what
        // later ones were allowed to do (D73). It is read once, before any of the batch is judged.
        if (!\array_key_exists($id, $this->newest)) {
            $this->newest[$id] = $this->operations->newestJudgedHlc($library);
        }
        if (null !== $this->newest[$id] && strcmp($operation->hlc, $this->newest[$id]) < 0) {
            $this->late[$id] = true;
        }

        $status = $this->judge($library, $operation);
        $this->operations->setStatus($library, $operation->id, $status);
        // After the status, so an operation of the fenced device already in the log is seen.
        if (OperationStatus::Applied === $status && $this->noteFences($library, $operation)) {
            $this->late[$id] = true;
        }

        return $status;
    }

    /** Judges the library again from its log if something taken since the last call arrived late where order decides. */
    public function settle(CloudLibrary $library): void
    {
        $id = $library->getId()->toRfc4122();
        if (isset($this->late[$id]) && new CatalogLibraryView($this->entityManager, $library)->judgedByOrder()) {
            $this->rebuild($library);
        }
        unset($this->late[$id], $this->newest[$id]);
    }

    /**
     * Empties the catalog of the library and applies its whole log again, in HLC order, judging each
     * operation against the roles in force at its moment (D73).
     */
    public function rebuild(CloudLibrary $library): void
    {
        // Every fence in the log is a candidate; only those set by an operation that is itself allowed
        // count (D84). Dropping one can make another operation a request or not, so replay until no
        // candidate is dropped. The set only shrinks, so it ends, and every device reaches the same result.
        $fences = [];
        foreach ($this->log($library) as $operation) {
            array_push($fences, ...$this->fencesOf($operation));
        }
        while (true) {
            $this->replay($library, $fences);
            $statuses = $this->operations->statusesOf($library, array_values(array_unique(array_column($fences, 'operation'))));
            $kept = array_values(array_filter($fences, static fn (array $fence): bool => OperationStatus::Applied === ($statuses[$fence['operation']] ?? null)));
            if (\count($kept) === \count($fences)) {
                return;
            }
            $fences = $kept;
        }
    }

    /** Forgets what a batch that failed left behind, here and in the merge, so the next library starts clean. */
    public function reset(): void
    {
        $this->merge->reset();
        $this->newest = $this->late = [];
    }

    private function judge(CloudLibrary $library, OperationPayload $operation): OperationStatus
    {
        if (!Permissions::isAllowed($operation, new CatalogLibraryView($this->entityManager, $library))) {
            return OperationStatus::Request;
        }
        $this->merge->apply($library, $operation);
        $this->merge->flush();

        return OperationStatus::Applied;
    }

    /**
     * One pass over the log with the given fences in force.
     *
     * @param list<FenceCandidate> $fences
     */
    private function replay(CloudLibrary $library, array $fences): void
    {
        $this->merge->clear($library);
        foreach ([Fence::class, FenceChain::class] as $class) {
            $this->entityManager->createQuery(\sprintf('DELETE FROM %s x WHERE x.cloudLibrary = :library', $class))
                ->setParameter('library', $library->getId(), 'uuid')->execute();
        }
        foreach ($fences as $fence) {
            $this->insertFence($library, $fence);
        }
        foreach ($this->log($library) as $operation) {
            $status = $this->judge($library, $operation);
            $this->operations->setStatus($library, $operation->id, $status);
        }
    }

    /**
     * The library's log in HLC order, read by stretches so a big library never has to fit in memory.
     *
     * @return \Generator<int, OperationPayload>
     */
    private function log(CloudLibrary $library): \Generator
    {
        $after = '';
        while ([] !== $page = $this->operations->findPage($library, $after, self::PAGE)) {
            foreach ($page as ['hlc' => $hlc, 'operation' => $operation]) {
                yield $this->reader->read($operation);
                $after = $hlc;
            }
        }
    }

    /**
     * The fences an operation sets (D84, D87): its changes to a device that are not deletes and carry a
     * `trusted_through` of text.
     *
     * @return list<FenceCandidate>
     */
    private function fencesOf(OperationPayload $operation): array
    {
        $fences = [];
        foreach ($operation->changes as $change) {
            $through = $change->fields['trusted_through'] ?? null;
            if (CatalogEntity::Device === $change->entity && ChangeAction::Delete !== $change->action && \is_string($through)) {
                $fences[] = ['operation' => $operation->id, 'device' => $change->id, 'through' => $through, 'until' => $operation->hlc];
            }
        }

        return $fences;
    }

    /**
     * Remembers the fences an applied operation sets (D84). Returns whether the log already holds an
     * operation the new fence puts under suspicion: judged before the fence was known, it has to be judged again.
     */
    private function noteFences(CloudLibrary $library, OperationPayload $operation): bool
    {
        $late = false;
        foreach ($this->fencesOf($operation) as $fence) {
            $this->insertFence($library, $fence);
            $through = Uuid::isValid($fence['through']) ? Uuid::fromString($fence['through']) : null;
            $late = $this->operations->hasOperationUnderFence($library, $fence['device'], $fence['until'], $through, Uuid::fromString($fence['operation'])) || $late;
        }

        return $late;
    }

    /**
     * Notes the fence and the operations it still counts. Worked out here, once, so judging each operation
     * of a long log never walks the chain again. If `through` is not in the log yet, the chain is empty and
     * the fence covers all of the device's operations before it: they are judged again when it arrives.
     *
     * @param FenceCandidate $fence
     */
    private function insertFence(CloudLibrary $library, array $fence): void
    {
        $operation = Uuid::fromString($fence['operation']);
        $known = $this->entityManager->createQuery(\sprintf('SELECT 1 FROM %s f WHERE f.cloudLibrary = :library AND f.operationId = :operation AND f.deviceId = :device', Fence::class))
            ->setParameter('library', $library->getId(), 'uuid')->setParameter('operation', $operation, 'uuid')->setParameter('device', $fence['device'])->getResult();
        if ([] !== $known) {
            return;
        }

        $noted = [new Fence($library, $operation, $fence['device'], $fence['through'], $fence['until'])];
        $previous = $this->operations->previousOf($library, $fence['device']);
        $seen = [];
        $current = Uuid::isValid($fence['through']) ? Uuid::fromString($fence['through'])->toRfc4122() : null;
        while (null !== $current && \array_key_exists($current, $previous) && !isset($seen[$current])) {
            $seen[$current] = true;
            $noted[] = new FenceChain($library, $operation, $fence['device'], Uuid::fromString($current));
            $current = $previous[$current];
        }
        foreach ($noted as $row) {
            $this->entityManager->persist($row);
        }
        $this->entityManager->flush();
        // Bulk deletes and the next flush know nothing of these objects: do not leave them managed.
        foreach ($noted as $row) {
            $this->entityManager->detach($row);
        }
    }
}
