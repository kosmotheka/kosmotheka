<?php

declare(strict_types=1);

namespace App\Tests\Support;

use App\Dto\ChangePayload;
use App\Dto\OperationPayload;
use App\Entity\CloudLibrary;
use App\Enum\OperationStatus;
use App\Repository\CloudOperationRepository;
use App\Sync\Permission\CatalogJudge;
use App\Sync\SchemaVersion;
use Symfony\Component\Uid\Uuid;

/**
 * Operations delivered to the judge one by one, as the push of 7.4.6 will: stored in the log, taken and
 * settled. For the tests of the permission rules, which need real operations in the log (the judge may
 * read it again) but not their signatures to be right: the judge only asks that there is one.
 */
final readonly class Judged
{
    /** Something with the shape of a signature: the judge does not check it, only that there is one. */
    public const string SIGNATURE = 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA';

    public function __construct(
        public CloudLibrary $library,
        private CatalogJudge $judge,
        private CloudOperationRepository $operations,
        private SchemaVersion $schema,
    ) {
    }

    /**
     * A signed-looking operation of `$device`, made at `$time`.
     *
     * @param list<ChangePayload> $changes
     */
    public static function op(int $time, string $device, array $changes, ?string $previous = null, ?string $id = null): OperationPayload
    {
        return new OperationPayload($id ?? Uuid::v7()->toRfc4122(), '01997c7e-0000-7000-8000-00000000aaaa', Ops::hlc($time, 0, $device), 'test.op', $changes, $previous, self::SIGNATURE);
    }

    /** Stores the operation, takes it and settles the library; returns what became of it in the end. */
    public function deliver(OperationPayload $operation): OperationStatus
    {
        $this->store($operation);
        $this->judge->take($this->library, $operation);
        $this->judge->settle($this->library);

        return $this->status($operation->id) ?? throw new \LogicException('The operation was not judged.');
    }

    /** Puts the operation in the log, not judged yet: what a push does before the judge sees it. */
    public function store(OperationPayload $operation): void
    {
        $stored = $this->operations->insertIfNew($this->library, $operation->id, explode('-', $operation->hlc, 3)[2], $operation->hlc, $this->schema->current(), json_encode($this->body($operation), \JSON_THROW_ON_ERROR), $operation->previous);
        \assert($stored);
    }

    public function status(string $operationId): ?OperationStatus
    {
        return $this->operations->statusesOf($this->library, [$operationId])[$operationId] ?? null;
    }

    /**
     * @return array<string, mixed>
     */
    private function body(OperationPayload $operation): array
    {
        return [
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
            'signature' => $operation->signature,
        ];
    }
}
