<?php

declare(strict_types=1);

namespace App\Sync\Merge;

use App\Dto\ChangePayload;
use App\Dto\FieldSpec;
use App\Dto\OperationPayload;
use App\Entity\Catalog\RowAuthor;
use App\Entity\Catalog\Tombstone;
use App\Entity\CloudLibrary;
use App\Enum\CatalogEntity;
use App\Enum\ChangeAction;
use App\Enum\ChangeOutcome;
use App\Enum\FieldType;
use Symfony\Component\Uid\Uuid;

/**
 * The merge rules of spec/README.md written with DBAL (D92, revised on 2026-10-09). The rules are those of
 * the ORM engine this replaces and keep its numbering; only the way of reading and writing changes.
 *
 * Every situation that would make the database refuse a row is judged BEFORE anything is written: in
 * PostgreSQL a failing statement leaves the transaction useless, so the database is the net below this code,
 * never what decides. Nothing the client sends is used as a class, property or column name, and a row is only
 * ever looked up together with its cloud library.
 *
 * The rules decide and queue statements; flush() runs them in order. Between two flushes the merge remembers,
 * of each row it touched, only what the rules need (its key, the version of each field, the keys it points
 * to and, for a contributor, its role) plus the tombstones it asked about, because a row that is still in
 * the queue is not yet in the database. The memory is forgotten on every flush. The merge opens no transaction.
 */
final class DbalCatalogMerge implements CatalogMerge
{
    /** Fields that travel in the operation but have no column (D84): only their version is kept. */
    private const array NOT_STORED = ['library' => ['cloud_library_id', 'cloud_server']];

    /** @var array<string, array<string, ?array{key: string, versions: array<string, string>, refs: array<string, ?string>, role: ?string}>> by cloud library, then by entity and id */
    private array $rows = [];

    /** @var array<string, array<string, bool>> by cloud library, then by entity and id */
    private array $tombstones = [];

    /** @var array<string, bool> by cloud library */
    private array $libraryDeleted = [];

    /** @var array<string, array<string, true>> rows of the cloud library whose author is known, by entity and id */
    private array $authors = [];

    /** @var list<SqlStatement> */
    private array $queue = [];

    public function __construct(private readonly CatalogSql $sql)
    {
    }

    public function apply(CloudLibrary $library, OperationPayload $operation): array
    {
        $id = $library->getId()->toRfc4122();

        return array_map(function (ChangePayload $change) use ($id, $operation): ChangeOutcome {
            $outcome = $this->change($id, $operation->hlc, $change);
            if (ChangeOutcome::Written === $outcome && ChangeAction::Create === $change->action) {
                $this->noteAuthor($id, $operation->hlc, $change);
            }

            return $outcome;
        }, $operation->changes);
    }

    public function flush(): void
    {
        $this->run();
        $this->rows = $this->tombstones = $this->libraryDeleted = $this->authors = [];
    }

    public function clear(CloudLibrary $library): void
    {
        $id = $library->getId()->toRfc4122();
        $this->wipe($id);
        unset($this->libraryDeleted[$id]);
    }

    private function change(string $library, string $hlc, ChangePayload $change): ChangeOutcome
    {
        // 1 and 2: the delete wins, whatever the HLC.
        if ($this->libraryDeleted($library) || $this->tombstone($library, $change->entity, $change->id)) {
            return ChangeOutcome::Skipped;
        }

        return ChangeAction::Delete === $change->action
            ? $this->delete($library, $hlc, $change)
            : $this->write($library, $hlc, $change);
    }

    /** Rules 3 and 4. */
    private function delete(string $library, string $hlc, ChangePayload $change): ChangeOutcome
    {
        $entity = $change->entity;
        if (CatalogEntity::Library === $entity) {
            $this->wipe($library);
            $this->libraryDeleted[$library] = true;
        } else {
            $this->queue[] = $this->sql->delete($entity->entityClass(), ['cloudLibrary' => $library, 'id' => $change->id]);
            // The database cascades and nulls references (D89): nothing read before this can be trusted.
            $this->run();
            $this->rows[$library] = $this->tombstones[$library] = [];
        }
        $this->queue[] = $this->sql->insert(Tombstone::class, ['cloudLibrary' => $library, 'entity' => $entity->value, 'entityId' => $change->id, 'hlc' => $hlc]);
        $this->tombstones[$library][$this->rowId($entity, $change->id)] = true;

        return ChangeOutcome::Written;
    }

    /** Rules 5 to 17: a create or an update. */
    private function write(string $library, string $hlc, ChangePayload $change): ChangeOutcome
    {
        $entity = $change->entity;
        $specs = $entity->fields();

        // Only the fields of the list. The loop runs over the entity's own keys; what the client wrote is only compared.
        $given = [];
        foreach ($specs as $name => $spec) {
            if (\array_key_exists($name, $change->fields)) {
                $given[$name] = $change->fields[$name];
            }
        }
        if (\count($given) !== \count($change->fields)) {
            return ChangeOutcome::Skipped;
        }

        $row = $this->row($library, $entity, $change->id);
        if (null === $row && ChangeAction::Update === $change->action) {
            return ChangeOutcome::Skipped; // 5
        }

        // A reference to a deleted row is null before any HLC is compared (6). One that points nowhere is only
        // a problem if its field is written, so it is remembered here and judged below, among the winners.
        $values = [];
        $unusable = [];
        foreach ($given as $name => $value) {
            if (FieldType::Ref === $specs[$name]->type && null !== $value) {
                $reference = $this->reference($library, $specs[$name], $value);
                $unusable[$name] = false === $reference;
                $value = false === $reference ? null : $reference;
            }
            $values[$name] = $value;
        }

        // 12 and 15: field by field, byte by byte; a tie is not newer.
        $versions = $row['versions'] ?? [];
        $won = [];
        foreach ($values as $name => $value) {
            if (!isset($versions[$name]) || strcmp($hlc, $versions[$name]) > 0) {
                $won[$name] = $value;
            }
        }

        // Only what is written can break a constraint (as in the Rust client): 7 and 8, NOT NULL and the column's type.
        foreach ($won as $name => $value) {
            $spec = $specs[$name];
            if ($unusable[$name] ?? false) {
                return ChangeOutcome::Skipped;
            }
            if (null === $value ? $spec->required : (FieldType::Ref !== $spec->type && !$this->holds($spec->type, $value))) {
                return ChangeOutcome::Skipped;
            }
        }

        $edition = null;
        if (null === $row) {
            // 10, and what is not written stays null: every required field has to be among those that won.
            foreach ($specs as $name => $spec) {
                if ($spec->required && null === ($won[$name] ?? null)) {
                    return ChangeOutcome::Skipped;
                }
            }
            if ($entity->isDetails()) {
                $edition = $this->row($library, CatalogEntity::Edition, $change->id)['key'] ?? null;
                if (null === $edition) {
                    return ChangeOutcome::Skipped; // 9
                }
            }
        } elseif ([] === $won) {
            return ChangeOutcome::Skipped; // 14
        }

        $role = $row['role'] ?? null;
        if (CatalogEntity::WorkContributor === $entity && [] !== array_intersect_key($won, ['work_id' => 0, 'person_id' => 0, 'role' => 0])) {
            $triple = [
                \array_key_exists('work_id', $won) ? $won['work_id'] : $row['refs']['work_id'] ?? null,
                \array_key_exists('person_id', $won) ? $won['person_id'] : $row['refs']['person_id'] ?? null,
                \array_key_exists('role', $won) ? $won['role'] : $role,
            ];
            if ($this->repeatsContributor($library, $change->id, $triple)) {
                return ChangeOutcome::Skipped; // 11
            }
            $role = \is_string($triple[2]) ? $triple[2] : null;
        }

        // Nothing below can be refused: write.
        $set = [];
        $refs = $row['refs'] ?? [];
        foreach ($won as $name => $value) {
            $versions[$name] = $hlc;
            if (\in_array($name, self::NOT_STORED[$entity->value] ?? [], true)) { // 17
                continue;
            }
            $set[$this->property($name, $specs[$name])] = $value;
            if (FieldType::Ref === $specs[$name]->type) {
                $refs[$name] = \is_string($value) ? $value : null;
            }
        }
        $set['versions'] = $versions;

        if (null === $row) {
            $key = Uuid::v7()->toRfc4122();
            if (null !== $edition) {
                $set['edition'] = $refs['edition'] = $edition;
            }
            $this->queue[] = $this->sql->insert($entity->entityClass(), ['key' => $key, 'cloudLibrary' => $library, 'id' => $change->id] + $set);
        } else {
            $key = $row['key'];
            $this->queue[] = $this->sql->update($entity->entityClass(), $set, ['key' => $key]);
        }
        $this->rows[$library][$this->rowId($entity, $change->id)] = ['key' => $key, 'versions' => $versions, 'refs' => $refs, 'role' => $role];

        return ChangeOutcome::Written;
    }

    /**
     * The key a reference names, null when it is optional and the row was deleted (6), or false when the
     * change cannot be applied: a row that does not exist, or a deleted one that is required (7, 8).
     */
    private function reference(string $library, FieldSpec $spec, mixed $id): string|false|null
    {
        if (!\is_string($id) || !$spec->references instanceof CatalogEntity) {
            return false;
        }
        $target = $this->row($library, $spec->references, $id);
        if (null !== $target) {
            return $target['key'];
        }

        return !$spec->required && $this->tombstone($library, $spec->references, $id) ? null : false;
    }

    /**
     * Rule 11. The (work, person, role) the row would end with, against every other row of the library. The
     * rows written before this one are still in the queue, so it runs first and the database answers.
     *
     * @param array{mixed, mixed, mixed} $triple
     */
    private function repeatsContributor(string $library, string $id, array $triple): bool
    {
        $this->run();

        return null !== $this->sql->first(
            CatalogEntity::WorkContributor->entityClass(),
            ['id'],
            ['cloudLibrary' => $library, 'work' => $triple[0], 'person' => $triple[1], 'role' => $triple[2]],
            ['id' => $id],
        );
    }

    /** @return ?array{key: string, versions: array<string, string>, refs: array<string, ?string>, role: ?string} */
    private function row(string $library, CatalogEntity $entity, string $id): ?array
    {
        $rowId = $this->rowId($entity, $id);
        if (\array_key_exists($rowId, $this->rows[$library] ?? [])) {
            return $this->rows[$library][$rowId];
        }

        $refs = [];
        foreach ($entity->fields() as $name => $spec) {
            if (FieldType::Ref === $spec->type) {
                $refs[$name] = $this->property($name, $spec);
            }
        }
        if ($entity->isDetails()) {
            $refs['edition'] = 'edition';
        }
        $found = $this->sql->first(
            $entity->entityClass(),
            ['key', 'versions', ...array_values($refs), ...(CatalogEntity::WorkContributor === $entity ? ['role'] : [])],
            ['cloudLibrary' => $library, 'id' => $id],
        );
        if (null === $found) {
            return $this->rows[$library][$rowId] = null;
        }

        $decoded = \is_string($found['versions']) ? json_decode($found['versions'], true, flags: \JSON_THROW_ON_ERROR) : null;
        if (!\is_string($found['key']) || !\is_array($decoded)) {
            throw new UnexpectedRowException(\sprintf('A %s row came back without a key or versions.', $entity->value));
        }
        $versions = [];
        foreach ($decoded as $field => $hlc) {
            if (!\is_string($hlc)) {
                throw new UnexpectedRowException(\sprintf('A %s row has a version that is not an HLC.', $entity->value));
            }
            $versions[(string) $field] = $hlc;
        }

        return $this->rows[$library][$rowId] = [
            'key' => $found['key'],
            'versions' => $versions,
            'refs' => array_map(static fn (string $property): ?string => \is_string($found[$property]) ? $found[$property] : null, $refs),
            'role' => \is_string($found['role'] ?? null) ? $found['role'] : null,
        ];
    }

    private function tombstone(string $library, CatalogEntity $entity, string $id): bool
    {
        $rowId = $this->rowId($entity, $id);

        return $this->tombstones[$library][$rowId] ??= null !== $this->sql->first(Tombstone::class, ['hlc'], ['cloudLibrary' => $library, 'entity' => $entity->value, 'entityId' => $id]);
    }

    private function libraryDeleted(string $library): bool
    {
        // One `library` row per cloud library: whatever id its tombstone carries, it is this library's.
        return $this->libraryDeleted[$library] ??= null !== $this->sql->first(Tombstone::class, ['hlc'], ['cloudLibrary' => $library, 'entity' => CatalogEntity::Library->value]);
    }

    /**
     * Remembers which device created a row (D45): the first create counts, a later create of the same row is an edit.
     * Whether one is noted already is asked of the database and remembered until the next flush, like a tombstone.
     */
    private function noteAuthor(string $library, string $hlc, ChangePayload $change): void
    {
        $rowId = $this->rowId($change->entity, $change->id);
        if (isset($this->authors[$library][$rowId])) {
            return;
        }
        $this->authors[$library][$rowId] = true;
        $known = null !== $this->sql->first(RowAuthor::class, ['deviceId'], ['cloudLibrary' => $library, 'entity' => $change->entity->value, 'entityId' => $change->id]);
        if (!$known) {
            $this->queue[] = $this->sql->insert(RowAuthor::class, [
                'cloudLibrary' => $library,
                'entity' => $change->entity->value,
                'entityId' => $change->id,
                'deviceId' => explode('-', $hlc, 3)[2] ?? '',
            ]);
        }
    }

    /** Everything of the library except the cloud library itself and its log. */
    private function wipe(string $library): void
    {
        foreach (CatalogEntity::cases() as $entity) {
            $this->queue[] = $this->sql->delete($entity->entityClass(), ['cloudLibrary' => $library]);
        }
        foreach ([Tombstone::class, RowAuthor::class] as $class) {
            $this->queue[] = $this->sql->delete($class, ['cloudLibrary' => $library]);
        }
        $this->run();
        $this->rows[$library] = $this->tombstones[$library] = $this->authors[$library] = [];
    }

    public function reset(): void
    {
        $this->queue = $this->rows = $this->tombstones = $this->libraryDeleted = $this->authors = [];
    }

    private function run(): void
    {
        $queue = $this->queue;
        $this->queue = [];
        foreach ($queue as $statement) {
            $this->sql->execute($statement);
        }
    }

    /** `work_id` is the property `work` and `published_year` is `publishedYear`. */
    private function property(string $name, FieldSpec $spec): string
    {
        $base = FieldType::Ref === $spec->type ? substr($name, 0, -3) : $name;

        return lcfirst(str_replace('_', '', ucwords($base, '_')));
    }

    private function rowId(CatalogEntity $entity, string $id): string
    {
        return $entity->value.'|'.$id;
    }

    private function holds(FieldType $type, mixed $value): bool
    {
        return match ($type) {
            // PostgreSQL text cannot hold U+0000 or invalid UTF-8.
            FieldType::Text, FieldType::Date, FieldType::OneOf, FieldType::Isbn13, FieldType::DeviceId, FieldType::DeviceIdOrEmpty => \is_string($value) && !str_contains($value, "\0") && mb_check_encoding($value, 'UTF-8'),
            FieldType::Integer, FieldType::Count => \is_int($value),
            // The column is an INTEGER and the spec says 1 to 5.
            FieldType::Rating => \is_int($value) && $value >= 1 && $value <= 5,
            FieldType::Boolean => \is_bool($value),
            FieldType::Ref => false,
        };
    }
}
