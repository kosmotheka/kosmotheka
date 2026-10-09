<?php

declare(strict_types=1);

namespace App\Sync\Merge;

use App\Dto\ChangePayload;
use App\Dto\FieldSpec;
use App\Dto\OperationPayload;
use App\Entity\Catalog\CatalogRow;
use App\Entity\Catalog\FieldVersion;
use App\Entity\Catalog\RowAuthor;
use App\Entity\Catalog\Tombstone;
use App\Entity\Catalog\WorkContributor;
use App\Entity\CloudLibrary;
use App\Enum\CatalogEntity;
use App\Enum\ChangeAction;
use App\Enum\ChangeOutcome;
use App\Enum\FieldType;
use Doctrine\ORM\EntityManagerInterface;

/**
 * The merge rules of spec/README.md written with the ORM (D92), driven by
 * CatalogEntity: the class of a row comes from entityClass(), its fields from
 * fields(), and values go in and out through Doctrine's metadata.
 *
 * Every situation that would make the database refuse a row is judged BEFORE
 * anything is persisted. A failing flush closes the EntityManager and loses the
 * whole request, so the database is the net below this code, never what decides.
 * Nothing the client sends is used as a class, property or column name, and a row
 * is only ever looked up together with its cloud library.
 *
 * What the merge holds in memory (rows, versions and tombstones touched since the
 * last flush) is looked at before the database, because Doctrine does not see
 * what is persisted but not yet flushed.
 */
final class OrmCatalogMerge implements CatalogMerge
{
    /** Fields that travel in the operation but have no column (D84): only their version is kept. */
    private const array NOT_STORED = ['library' => ['cloud_library_id', 'cloud_server']];

    /** @var array<string, ?CatalogRow> */
    private array $rows = [];

    /** @var array<string, array<string, FieldVersion>> per row, by field */
    private array $versions = [];

    /** @var array<string, ?Tombstone> */
    private array $tombstones = [];

    /** @var array<string, bool> by cloud library */
    private array $libraryDeleted = [];

    public function __construct(private readonly EntityManagerInterface $entityManager)
    {
    }

    public function apply(CloudLibrary $library, OperationPayload $operation): array
    {
        return array_map(fn (ChangePayload $change): ChangeOutcome => $this->change($library, $operation->hlc, $change), $operation->changes);
    }

    public function flush(): void
    {
        $this->entityManager->flush();
        $this->forget();
    }

    public function clear(CloudLibrary $library): void
    {
        $this->entityManager->flush();
        $this->wipe($library);
        unset($this->libraryDeleted[$this->libraryKey($library)]);
        $this->forget();
    }

    private function change(CloudLibrary $library, string $hlc, ChangePayload $change): ChangeOutcome
    {
        // 1 and 2: the delete wins, whatever the HLC.
        if ($this->libraryDeleted($library) || $this->tombstone($library, $change->entity, $change->id) instanceof Tombstone) {
            return ChangeOutcome::Skipped;
        }

        return ChangeAction::Delete === $change->action
            ? $this->delete($library, $hlc, $change)
            : $this->write($library, $hlc, $change);
    }

    /** Rules 3 and 4. */
    private function delete(CloudLibrary $library, string $hlc, ChangePayload $change): ChangeOutcome
    {
        // The bulk deletes below see only the database, and the database cascades and nulls references (D89).
        $this->entityManager->flush();

        if (CatalogEntity::Library === $change->entity) {
            $this->wipe($library);
            $this->libraryDeleted[$this->libraryKey($library)] = true;
        } else {
            $this->deleteWhere(FieldVersion::class, $library, ['entity' => $change->entity->value, 'entityId' => $change->id]);
            $this->deleteWhere($change->entity->entityClass(), $library, ['id' => $change->id]);
        }

        // What was loaded may have been cascaded away or had a reference nulled: none of it can be trusted now.
        $this->forget();

        $tombstone = new Tombstone($library, $change->entity->value, $change->id, $hlc);
        $this->entityManager->persist($tombstone);
        $this->tombstones[$this->rowKey($library, $change->entity, $change->id)] = $tombstone;

        return ChangeOutcome::Written;
    }

    /** Rules 5 to 17: a create or an update. */
    private function write(CloudLibrary $library, string $hlc, ChangePayload $change): ChangeOutcome
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
        if (!$row instanceof CatalogRow && ChangeAction::Update === $change->action) {
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
        $versions = $this->versionsOf($library, $entity, $change->id);
        $won = [];
        foreach ($values as $name => $value) {
            $stored = $versions[$name] ?? null;
            if (null === $stored || strcmp($hlc, $stored->getHlc()) > 0) {
                $won[$name] = $value;
            }
        }

        // Only what is written can break a constraint (as in the Rust client): 7 and 8, NOT NULL and the column's type.
        foreach ($won as $name => $value) {
            $spec = $specs[$name];
            if ($unusable[$name] ?? false) {
                return ChangeOutcome::Skipped;
            }
            if (null === $value ? $spec->required : (FieldType::Ref !== $spec->type && !self::holds($spec->type, $value))) {
                return ChangeOutcome::Skipped;
            }
        }

        $edition = null;
        if (!$row instanceof CatalogRow) {
            // 10, and what is not written stays null: every required field has to be among those that won.
            foreach ($specs as $name => $spec) {
                if ($spec->required && null === ($won[$name] ?? null)) {
                    return ChangeOutcome::Skipped;
                }
            }
            if ($entity->isDetails()) {
                $edition = $this->row($library, CatalogEntity::Edition, $change->id);
                if (!$edition instanceof CatalogRow) {
                    return ChangeOutcome::Skipped; // 9
                }
            }
        } elseif ([] === $won) {
            return ChangeOutcome::Skipped; // 14
        }

        $metadata = $this->entityManager->getClassMetadata($entity->entityClass());
        $retriple = false;
        if (CatalogEntity::WorkContributor === $entity && [] !== array_intersect_key($won, ['work_id' => 0, 'person_id' => 0, 'role' => 0])) {
            if ($this->repeatsContributor($library, $row, $won)) {
                return ChangeOutcome::Skipped; // 11
            }
            $retriple = $row instanceof CatalogRow;
        }

        // Nothing below can be refused: write.
        if (!$row instanceof CatalogRow) {
            $class = $entity->entityClass();
            $row = new $class($library, $change->id);
            if ($edition instanceof CatalogRow) {
                $metadata->setFieldValue($row, 'edition', $edition);
            }
            $this->entityManager->persist($row);
            $this->rows[$this->rowKey($library, $entity, $change->id)] = $row;
        }
        foreach ($won as $name => $value) {
            if (!\in_array($name, self::NOT_STORED[$entity->value] ?? [], true)) { // 17
                $metadata->setFieldValue($row, $this->property($name, $specs[$name]), $value);
            }
            if (null === ($version = $versions[$name] ?? null)) {
                $version = new FieldVersion($library, $entity->value, $change->id, $name, $hlc);
                $this->entityManager->persist($version);
                $this->versions[$this->rowKey($library, $entity, $change->id)][$name] = $version;
            } else {
                $version->setHlc($hlc);
            }
        }

        if ($retriple) {
            // Two rows swapping the same triple inside one flush could meet halfway: the database sees them one by one.
            $this->entityManager->flush();
        }

        return ChangeOutcome::Written;
    }

    /**
     * The row a reference names, null when it is optional and the row was deleted (6), or false when the
     * change cannot be applied: a row that does not exist, or a deleted one that is required (7, 8).
     */
    private function reference(CloudLibrary $library, FieldSpec $spec, mixed $id): CatalogRow|false|null
    {
        if (!\is_string($id) || !$spec->references instanceof CatalogEntity) {
            return false;
        }
        $target = $this->row($library, $spec->references, $id);
        if ($target instanceof CatalogRow) {
            return $target;
        }

        return !$spec->required && $this->tombstone($library, $spec->references, $id) instanceof Tombstone ? null : false;
    }

    /**
     * Rule 11. The final (work, person, role) of the row, against every other row of the library: the
     * flushed ones, which the database knows, and the pending ones, which only the merge knows.
     *
     * @param array<string, mixed> $won
     */
    private function repeatsContributor(CloudLibrary $library, ?CatalogRow $row, array $won): bool
    {
        $current = $row instanceof WorkContributor ? $row : null;
        $work = $won['work_id'] ?? $current?->getWork();
        $person = $won['person_id'] ?? $current?->getPerson();
        $role = $won['role'] ?? $current?->getRole();

        $repository = $this->entityManager->getRepository(WorkContributor::class);
        $others = $repository->findBy(['cloudLibrary' => $library, 'work' => $work, 'person' => $person, 'role' => $role]);
        foreach ($this->rows as $pending) {
            if ($pending instanceof WorkContributor && !\in_array($pending, $others, true)) {
                $others[] = $pending;
            }
        }

        return array_any($others, static fn (WorkContributor $other): bool => $other !== $current && $other->getWork() === $work && $other->getPerson() === $person && $other->getRole() === $role);
    }

    private function row(CloudLibrary $library, CatalogEntity $entity, string $id): ?CatalogRow
    {
        $key = $this->rowKey($library, $entity, $id);
        if (\array_key_exists($key, $this->rows)) {
            return $this->rows[$key];
        }

        return $this->rows[$key] = $this->entityManager->getRepository($entity->entityClass())->findOneBy(['cloudLibrary' => $library, 'id' => $id]);
    }

    private function tombstone(CloudLibrary $library, CatalogEntity $entity, string $id): ?Tombstone
    {
        $key = $this->rowKey($library, $entity, $id);
        if (\array_key_exists($key, $this->tombstones)) {
            return $this->tombstones[$key];
        }

        return $this->tombstones[$key] = $this->entityManager->getRepository(Tombstone::class)
            ->findOneBy(['cloudLibrary' => $library, 'entity' => $entity->value, 'entityId' => $id]);
    }

    private function libraryDeleted(CloudLibrary $library): bool
    {
        // One `library` row per cloud library: whatever id its tombstone carries, it is this library's.
        return $this->libraryDeleted[$this->libraryKey($library)] ??= null !== $this->entityManager->getRepository(Tombstone::class)
            ->findOneBy(['cloudLibrary' => $library, 'entity' => CatalogEntity::Library->value]);
    }

    /** @return array<string, FieldVersion> by field */
    private function versionsOf(CloudLibrary $library, CatalogEntity $entity, string $id): array
    {
        $key = $this->rowKey($library, $entity, $id);
        if (!isset($this->versions[$key])) {
            $this->versions[$key] = [];
            foreach ($this->entityManager->getRepository(FieldVersion::class)->findBy(['cloudLibrary' => $library, 'entity' => $entity->value, 'entityId' => $id]) as $version) {
                $this->versions[$key][$version->getField()] = $version;
            }
        }

        return $this->versions[$key];
    }

    /** Everything of the library except the cloud library itself and its log. */
    private function wipe(CloudLibrary $library): void
    {
        foreach (CatalogEntity::cases() as $entity) {
            $this->deleteWhere($entity->entityClass(), $library);
        }
        foreach ([FieldVersion::class, Tombstone::class, RowAuthor::class] as $class) {
            $this->deleteWhere($class, $library);
        }
    }

    /**
     * Bulk delete of the library's rows of a class, narrowed by properties of the class (never by client text).
     *
     * @param class-string         $class
     * @param array<string, mixed> $match property => value
     */
    private function deleteWhere(string $class, CloudLibrary $library, array $match = []): void
    {
        $query = $this->entityManager->createQueryBuilder()
            ->delete($class, 'x')
            ->where('x.cloudLibrary = :library')
            ->setParameter('library', $library);
        foreach ($match as $property => $value) {
            $query->andWhere(\sprintf('x.%1$s = :%1$s', $property))->setParameter($property, $value);
        }
        $query->getQuery()->execute();
    }

    /** Detaches what the merge loaded or created, and empties its memory. */
    private function forget(): void
    {
        foreach ($this->rows as $row) {
            if (null !== $row) {
                $this->entityManager->detach($row);
            }
        }
        foreach ($this->versions as $versions) {
            foreach ($versions as $version) {
                $this->entityManager->detach($version);
            }
        }
        foreach ($this->tombstones as $tombstone) {
            if (null !== $tombstone) {
                $this->entityManager->detach($tombstone);
            }
        }
        $this->rows = [];
        $this->versions = [];
        $this->tombstones = [];
    }

    /** `work_id` is the property `work` and `published_year` is `publishedYear`. */
    private function property(string $name, FieldSpec $spec): string
    {
        $base = FieldType::Ref === $spec->type ? substr($name, 0, -3) : $name;

        return lcfirst(str_replace('_', '', ucwords($base, '_')));
    }

    private function holds(FieldType $type, mixed $value): bool
    {
        return match ($type) {
            // PostgreSQL text cannot hold U+0000 or invalid UTF-8.
            FieldType::Text, FieldType::Date, FieldType::OneOf, FieldType::Isbn13 => \is_string($value) && !str_contains($value, "\0") && mb_check_encoding($value, 'UTF-8'),
            FieldType::Integer, FieldType::Count => \is_int($value),
            // The column is an INTEGER and the spec says 1 to 5.
            FieldType::Rating => \is_int($value) && $value >= 1 && $value <= 5,
            FieldType::Boolean => \is_bool($value),
            FieldType::Ref => false,
        };
    }

    private function libraryKey(CloudLibrary $library): string
    {
        return $library->getId()->toRfc4122();
    }

    private function rowKey(CloudLibrary $library, CatalogEntity $entity, string $id): string
    {
        return $this->libraryKey($library).'|'.$entity->value.'|'.$id;
    }
}
