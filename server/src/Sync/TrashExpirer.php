<?php

declare(strict_types=1);

namespace App\Sync;

use App\Entity\Catalog\TrashSeen;
use App\Entity\CloudLibrary;
use App\Enum\CatalogEntity;
use App\Sync\Permission\Permissions;
use App\Validator\ValidOperationValidator;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;
use Symfony\Component\Uid\Uuid;

/**
 * The cloud's safety net for the trash (D105, rule 16 of spec/README.md): in each library where the cloud has its
 * power, it deletes for good, signed as its device, what has been 60 days in the trash, counted from the later of
 * `trashed_at` and when the row reached this server (`TrashSeen`). The devices do it at 30 days; this only catches
 * what nobody opened in time, so nothing stays for ever.
 *
 * It signs only what rule 16 allows the cloud (everything it deletes is, or hangs from, a row 60 days in the trash
 * by the server's count, which is never earlier than `trashed_at`), so it leaves no request in the log. The
 * deletes are what deleting was before the trash, children first, as the app writes them (`trash::purge_changes`).
 */
final readonly class TrashExpirer
{
    private const int DAY_MILLIS = 24 * 60 * 60 * 1000;

    /** The four things of the trash, with their Doctrine class. */
    private const array TRASHABLE = [CatalogEntity::Edition, CatalogEntity::Copy, CatalogEntity::Tag, CatalogEntity::Series];

    public function __construct(
        private EntityManagerInterface $entityManager,
        private CloudAuthor $author,
        private SyncNotifier $notifier,
        private ClockInterface $clock,
    ) {
    }

    /** @return int how many things went for good, in every library where the cloud has its power */
    public function expire(): int
    {
        $gone = 0;
        foreach ($this->entityManager->getRepository(CloudLibrary::class)->findAll() as $library) {
            if (!$this->author->power($library) instanceof Uuid) {
                continue;
            }
            try {
                $gone += $this->expireIn($library);
            } catch (CloudAuthorException) {
                // Without its key here (D95), the cloud signs nothing: the devices still empty the trash.
            }
        }

        return $gone;
    }

    /** @return int how many things went */
    public function expireIn(CloudLibrary $library): int
    {
        $now = (int) $this->clock->now()->format('Uv');
        $due = $this->due($library, $now);
        if ([] === $due) {
            return 0;
        }
        $changes = [];
        foreach ($due as [$entity, $id]) {
            foreach ($this->removal($library, $entity, $id) as $change) {
                $changes[$change['entity'].'|'.$change['id']] ??= $change;
            }
        }
        $this->entityManager->wrapInTransaction(fn (): string => $this->author->author($library, 'trash.expire', array_values($changes)));
        $this->notifier->libraryChanged($library);
        $this->observe($library, $now);

        return \count($due);
    }

    /**
     * What has been 60 days in the trash of `$library`, after noting what it sees there for the first time.
     *
     * @return list<array{CatalogEntity, string}>
     */
    private function due(CloudLibrary $library, int $now): array
    {
        $seen = $this->observe($library, $now);
        $due = [];
        foreach ($this->trashed($library) as $key => [$entity, $id, $trashedAt]) {
            $from = max(ValidOperationValidator::timestampMillis($trashedAt) ?? 0, $seen[$key] ?? $now);
            if ($now >= $from + Permissions::CLOUD_TRASH_DAYS * self::DAY_MILLIS) {
                $due[] = [$entity, $id];
            }
        }

        return $due;
    }

    /**
     * Notes when this server first sees each thing in the trash and forgets what is no longer there (restored or gone).
     *
     * @return array<string, int> when each thing in the trash was first seen, by `entity|id`
     */
    private function observe(CloudLibrary $library, int $now): array
    {
        $trashed = $this->trashed($library);
        $seen = [];
        $rows = $this->entityManager->createQuery(\sprintf('SELECT s FROM %s s WHERE s.cloudLibrary = :library', TrashSeen::class))
            ->setParameter('library', $library->getId(), 'uuid')->getResult();
        foreach ($rows as $row) {
            $key = $row->getEntity().'|'.$row->getEntityId();
            if (isset($trashed[$key])) {
                $seen[$key] = $row->getSeenAt();
            } else {
                $this->entityManager->remove($row);
            }
        }
        foreach ($trashed as $key => [$entity, $id]) {
            if (!isset($seen[$key])) {
                $this->entityManager->persist(new TrashSeen($library, $entity->value, $id, $now));
                $seen[$key] = $now;
            }
        }
        $this->entityManager->flush();

        return $seen;
    }

    /**
     * Every row in the trash of `$library`, by `entity|id`.
     *
     * @return array<string, array{CatalogEntity, string, string}>
     */
    private function trashed(CloudLibrary $library): array
    {
        $trashed = [];
        foreach (self::TRASHABLE as $entity) {
            $rows = $this->entityManager->createQuery(\sprintf('SELECT r.id, r.trashedAt FROM %s r WHERE r.cloudLibrary = :library AND r.trashedAt IS NOT NULL', $entity->entityClass()))
                ->setParameter('library', $library->getId(), 'uuid')->getArrayResult();
            foreach ($rows as $row) {
                if (\is_array($row) && \is_string($row['id'] ?? null) && \is_string($row['trashedAt'] ?? null)) {
                    $trashed[$entity->value.'|'.$row['id']] = [$entity, $row['id'], $row['trashedAt']];
                }
            }
        }

        return $trashed;
    }

    /**
     * What deleting one thing for good takes with it, children first: what deleting it took before the trash.
     *
     * @return list<array{entity: string, id: string, action: string}>
     */
    private function removal(CloudLibrary $library, CatalogEntity $entity, string $id): array
    {
        $delete = static fn (CatalogEntity $entity, string $id): array => ['entity' => $entity->value, 'id' => $id, 'action' => 'delete'];
        $linked = fn (CatalogEntity $from, string $association, string $value): array => array_map(
            static fn (string $row): array => $delete($from, $row),
            $this->idsThrough($library, $from, $association, $value),
        );

        $changes = match ($entity) {
            CatalogEntity::Tag => $linked(CatalogEntity::WorkTag, 'tag', $id),
            CatalogEntity::Series => $linked(CatalogEntity::SeriesEntry, 'series', $id),
            CatalogEntity::Edition => $this->editionRemoval($library, $id, $delete, $linked),
            default => [],
        };
        if (CatalogEntity::Edition !== $entity) {
            $changes[] = $delete($entity, $id);
        }

        return $changes;
    }

    /**
     * An edition with its copies and details, and its work with its links if no other edition is left (`book_removal`).
     *
     * @param \Closure(CatalogEntity, string): array{entity: string, id: string, action: string}               $delete
     * @param \Closure(CatalogEntity, string, string): list<array{entity: string, id: string, action: string}> $linked
     *
     * @return list<array{entity: string, id: string, action: string}>
     */
    private function editionRemoval(CloudLibrary $library, string $id, \Closure $delete, \Closure $linked): array
    {
        $changes = $linked(CatalogEntity::Copy, 'edition', $id);
        foreach (CatalogEntity::cases() as $details) {
            if ($details->isDetails() && $this->exists($library, $details, $id)) {
                $changes[] = $delete($details, $id);
            }
        }
        $changes[] = $delete(CatalogEntity::Edition, $id);

        $work = $this->idsThrough($library, CatalogEntity::Edition, 'id', $id, 'work')[0] ?? null;
        if (null === $work || [] !== array_diff($this->idsThrough($library, CatalogEntity::Edition, 'work', $work), [$id])) {
            return $changes;
        }
        foreach ([CatalogEntity::WorkContributor, CatalogEntity::WorkTag, CatalogEntity::WorkGenre, CatalogEntity::SeriesEntry, CatalogEntity::WorkRelation] as $link) {
            array_push($changes, ...$linked($link, 'work', $work));
        }
        array_push($changes, ...$linked(CatalogEntity::WorkRelation, 'relatedWork', $work));
        $changes[] = $delete(CatalogEntity::Work, $work);

        return $changes;
    }

    /**
     * The app ids of the rows of `$entity` whose `$association` is the row `$value` (or, with `$select`, the ids of
     * what those rows point to through it).
     *
     * @return list<string>
     */
    private function idsThrough(CloudLibrary $library, CatalogEntity $entity, string $association, string $value, ?string $select = null): array
    {
        $match = 'id' === $association ? 'r.id = :value' : 'x.id = :value';
        $join = 'id' === $association ? '' : \sprintf('JOIN r.%s x', $association);
        $pick = null === $select ? 'r.id' : 's.id';
        $joinSelect = null === $select ? '' : \sprintf('JOIN r.%s s', $select);
        $rows = $this->entityManager->createQuery(\sprintf(
            'SELECT %s AS id FROM %s r %s %s WHERE r.cloudLibrary = :library AND %s ORDER BY r.id',
            $pick,
            $entity->entityClass(),
            $join,
            $joinSelect,
            $match,
        ))->setParameter('library', $library->getId(), 'uuid')->setParameter('value', $value)->getArrayResult();

        return array_values(array_filter(array_column($rows, 'id'), \is_string(...)));
    }

    private function exists(CloudLibrary $library, CatalogEntity $entity, string $id): bool
    {
        return [] !== $this->entityManager->createQuery(\sprintf('SELECT 1 FROM %s r WHERE r.cloudLibrary = :library AND r.id = :id', $entity->entityClass()))
            ->setParameter('library', $library->getId(), 'uuid')->setParameter('id', $id)->setMaxResults(1)->getResult();
    }
}
