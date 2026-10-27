<?php

declare(strict_types=1);

namespace App\Sync\Permission;

use App\Dto\OperationPayload;
use App\Entity\Catalog\Copy;
use App\Entity\Catalog\Device;
use App\Entity\Catalog\Edition;
use App\Entity\Catalog\Fence;
use App\Entity\Catalog\FenceChain;
use App\Entity\Catalog\Library;
use App\Entity\Catalog\Mark;
use App\Entity\Catalog\Member;
use App\Entity\Catalog\Revocation;
use App\Entity\Catalog\RowAuthor;
use App\Entity\Catalog\Series;
use App\Entity\Catalog\Tag;
use App\Entity\CloudLibrary;
use App\Entity\CloudOperation;
use App\Enum\CatalogEntity;
use App\Enum\Role;
use App\Sync\CloudDeviceId;
use App\Validator\ValidOperationValidator;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Uid\Uuid;

/**
 * The library of ONE cloud library as the permission rules see it (port of `SqlView` in
 * app/src-tauri/src/permissions.rs).
 *
 * Every query carries the cloud library: an id of a device, member, mark or operation is only ever looked
 * up inside it (D84). And nothing here loads a catalog entity: the merge engine writes with DBAL, below
 * the ORM, so an entity in memory may be out of date. Everything is a DQL query for scalars. The caller
 * makes sure the merge has nothing queued (`CatalogMerge::flush()`) before asking.
 */
final readonly class CatalogLibraryView implements LibraryView
{
    /**
     * A device belongs to its person only if it carries no `vouched_key`, or the one it is registered with (D96), and the person still has access (D98).
     * Byte by byte, like `public_key`.
     */
    private const string VOUCHED = " AND (d.vouchedKey IS NULL OR d.vouchedKey = '' OR d.vouchedKey = d.publicKey) AND (m.accessWithdrawn IS NULL OR m.accessWithdrawn = false)";

    public function __construct(
        private EntityManagerInterface $entityManager,
        private CloudLibrary $library,
    ) {
    }

    public function founded(): bool
    {
        return $this->exists(\sprintf('SELECT 1 FROM %s m WHERE m.cloudLibrary = :library AND m.role IS NOT NULL', Member::class));
    }

    public function roleOfDevice(string $device): ?Role
    {
        $role = $this->entityManager->createQuery(\sprintf(
            'SELECT m.role FROM %s d JOIN d.member m WHERE d.cloudLibrary = :library AND m.cloudLibrary = :library AND d.id = :device'.
            self::VOUCHED,
            Device::class,
        ))->setParameter('library', $this->library->getId(), 'uuid')->setParameter('device', $device)->setMaxResults(1)->getOneOrNullResult();

        return \is_array($role) && \is_string($role['role']) ? Role::tryFrom($role['role']) : null;
    }

    public function createdBySamePerson(CatalogEntity $entity, string $id, string $device): bool
    {
        $creator = $this->entityManager->createQuery(\sprintf(
            'SELECT a.deviceId FROM %s a WHERE a.cloudLibrary = :library AND a.entity = :entity AND a.entityId = :id',
            RowAuthor::class,
        ))->setParameter('library', $this->library->getId(), 'uuid')->setParameter('entity', $entity->value)->setParameter('id', $id)->setMaxResults(1)->getOneOrNullResult();

        return \is_array($creator) && \is_string($creator['deviceId']) && $this->samePerson($creator['deviceId'], $device);
    }

    public function requestedBySamePerson(string $request, string $device): bool
    {
        if (!Uuid::isValid($request)) {
            return false; // operations are named by UUIDs
        }
        $asker = $this->entityManager->createQuery(\sprintf(
            'SELECT o.authorDeviceId FROM %s o WHERE o.cloudLibrary = :library AND o.operationId = :id',
            CloudOperation::class,
        ))->setParameter('library', $this->library->getId(), 'uuid')->setParameter('id', Uuid::fromString($request), 'uuid')->setMaxResults(1)->getOneOrNullResult();

        return \is_array($asker) && \is_string($asker['authorDeviceId']) && $this->samePerson($asker['authorDeviceId'], $device);
    }

    public function memberOfDevice(string $device): ?string
    {
        $member = $this->entityManager->createQuery(\sprintf(
            'SELECT m.id FROM %s d JOIN d.member m WHERE d.cloudLibrary = :library AND m.cloudLibrary = :library AND d.id = :device'.
            self::VOUCHED,
            Device::class,
        ))->setParameter('library', $this->library->getId(), 'uuid')->setParameter('device', $device)->setMaxResults(1)->getOneOrNullResult();

        return \is_array($member) && \is_string($member['id']) ? $member['id'] : null;
    }

    public function rawMemberOfDevice(string $device): ?string
    {
        $member = $this->entityManager->createQuery(\sprintf(
            'SELECT m.id FROM %s d JOIN d.member m WHERE d.cloudLibrary = :library AND m.cloudLibrary = :library AND d.id = :device',
            Device::class,
        ))->setParameter('library', $this->library->getId(), 'uuid')->setParameter('device', $device)->setMaxResults(1)->getOneOrNullResult();

        return \is_array($member) && \is_string($member['id']) ? $member['id'] : null;
    }

    public function deviceHasMemberId(string $device): bool
    {
        return [] !== $this->entityManager->createQuery(\sprintf('SELECT 1 FROM %s d WHERE d.cloudLibrary = :library AND d.id = :device AND d.member IS NOT NULL', Device::class))
            ->setParameter('library', $this->library->getId(), 'uuid')->setParameter('device', $device)->setMaxResults(1)->getResult();
    }

    public function memberAccessWithdrawn(string $member): bool
    {
        return [] !== $this->entityManager->createQuery(\sprintf('SELECT 1 FROM %s m WHERE m.cloudLibrary = :library AND m.id = :member AND m.accessWithdrawn = true', Member::class))
            ->setParameter('library', $this->library->getId(), 'uuid')->setParameter('member', $member)->setMaxResults(1)->getResult();
    }

    public function markMember(string $id): ?string
    {
        $member = $this->entityManager->createQuery(\sprintf(
            'SELECT m.id FROM %s k JOIN k.member m WHERE k.cloudLibrary = :library AND m.cloudLibrary = :library AND k.id = :id',
            Mark::class,
        ))->setParameter('library', $this->library->getId(), 'uuid')->setParameter('id', $id)->setMaxResults(1)->getOneOrNullResult();

        return \is_array($member) && \is_string($member['id']) ? $member['id'] : null;
    }

    public function fenced(OperationPayload $operation): bool
    {
        $device = $operation->author();

        // Under the fence of an operation that took the power away: dated before it, and neither the last
        // operation the administrator knew nor one of its ancestors.
        $fences = $this->entityManager->createQuery(\sprintf(
            'SELECT 1 FROM %s f WHERE f.cloudLibrary = :library AND f.deviceId = :device AND :hlc < f.untilHlc AND :operationText <> f.through
               AND NOT EXISTS (SELECT 1 FROM %s c WHERE c.cloudLibrary = :library AND c.fenceOperationId = f.operationId
                                AND c.deviceId = :device AND c.operationId = :operation)',
            Fence::class,
            FenceChain::class,
        ))->setParameter('library', $this->library->getId(), 'uuid')->setParameter('device', $device)
            ->setParameter('hlc', $operation->hlc)->setParameter('operationText', $operation->id)->setParameter('operation', Uuid::fromString($operation->id), 'uuid')->setMaxResults(1)->getResult();
        if ([] !== $fences) {
            return true;
        }

        // After a revocation of the device in force at its HLC (D71): the version of the revocation's
        // `device_id` is the moment it was written.
        $revocations = $this->entityManager->createQuery(\sprintf(
            'SELECT r.versions FROM %s r WHERE r.cloudLibrary = :library AND r.deviceId = :device',
            Revocation::class,
        ))->setParameter('library', $this->library->getId(), 'uuid')->setParameter('device', $device)->getScalarResult();
        foreach ($revocations as $revocation) {
            // A scalar query hands the JSON column back as text.
            $versions = \is_array($revocation) && \is_string($revocation['versions'] ?? null) ? json_decode($revocation['versions'], true, flags: \JSON_THROW_ON_ERROR) : null;
            $since = \is_array($versions) ? ($versions['device_id'] ?? null) : null;
            if (\is_string($since) && strcmp($since, $operation->hlc) < 0) {
                return true;
            }
        }

        return false;
    }

    /** The id `library.cloud_device_id` holds now, whether or not that device is registered with the right key. */
    public function namedCloudDevice(): ?string
    {
        $row = $this->entityManager->createQuery(\sprintf(
            'SELECT l.cloudDeviceId FROM %s l WHERE l.cloudLibrary = :library',
            Library::class,
        ))->setParameter('library', $this->library->getId(), 'uuid')->setMaxResults(1)->getOneOrNullResult();

        return \is_array($row) && \is_string($row['cloudDeviceId']) && '' !== $row['cloudDeviceId'] ? $row['cloudDeviceId'] : null;
    }

    public function cloudDevice(): ?string
    {
        $named = $this->namedCloudDevice();
        if (null === $named) {
            return null;
        }

        // Only if the key registered for that device in the library is the one its id derives from (D95).
        $key = $this->entityManager->createQuery(\sprintf(
            'SELECT d.publicKey FROM %s d WHERE d.cloudLibrary = :library AND d.id = :device',
            Device::class,
        ))->setParameter('library', $this->library->getId(), 'uuid')->setParameter('device', $named)->setMaxResults(1)->getOneOrNullResult();
        $derived = \is_array($key) && \is_string($key['publicKey']) ? CloudDeviceId::fromPublicKey($key['publicKey']) : null;

        return null !== $derived && $derived === $named ? $named : null;
    }

    public function memberExists(string $member): bool
    {
        return [] !== $this->entityManager->createQuery(\sprintf('SELECT 1 FROM %s m WHERE m.cloudLibrary = :library AND m.id = :member', Member::class))
            ->setParameter('library', $this->library->getId(), 'uuid')->setParameter('member', $member)->setMaxResults(1)->getResult();
    }

    public function memberHasAccount(string $member): bool
    {
        return [] !== $this->entityManager->createQuery(\sprintf("SELECT 1 FROM %s m WHERE m.cloudLibrary = :library AND m.id = :member AND m.accountId IS NOT NULL AND m.accountId <> ''", Member::class))
            ->setParameter('library', $this->library->getId(), 'uuid')->setParameter('member', $member)->setMaxResults(1)->getResult();
    }

    public function trashedSince(CatalogEntity $entity, string $id): ?int
    {
        $work = fn (?string $work): ?int => null === $work ? null : $this->workTrashed($work);
        $through = fn (CatalogEntity $from, string $association): ?string => $this->linked($from, $id, $association);

        return match (true) {
            CatalogEntity::Edition === $entity, $entity->isDetails() => $this->ownTrashed(Edition::class, $id),
            CatalogEntity::Copy === $entity => $this->oldest($this->ownTrashed(Copy::class, $id), null === ($edition = $through($entity, 'edition')) ? null : $this->ownTrashed(Edition::class, $edition)),
            CatalogEntity::Tag === $entity => $this->ownTrashed(Tag::class, $id),
            CatalogEntity::Series === $entity => $this->ownTrashed(Series::class, $id),
            CatalogEntity::Work === $entity => $this->workTrashed($id),
            CatalogEntity::WorkContributor === $entity, CatalogEntity::WorkGenre === $entity => $work($through($entity, 'work')),
            CatalogEntity::WorkTag === $entity => $this->oldest($work($through($entity, 'work')), null === ($tag = $through($entity, 'tag')) ? null : $this->ownTrashed(Tag::class, $tag)),
            CatalogEntity::SeriesEntry === $entity => $this->oldest($work($through($entity, 'work')), null === ($series = $through($entity, 'series')) ? null : $this->ownTrashed(Series::class, $series)),
            CatalogEntity::WorkRelation === $entity => $this->oldest($work($through($entity, 'work')), $work($through($entity, 'relatedWork'))),
            default => null,
        };
    }

    /**
     * Since when a row of `$class` is in the trash, by its own `trashed_at`.
     *
     * @param class-string $class
     */
    private function ownTrashed(string $class, string $id): ?int
    {
        $row = $this->entityManager->createQuery(\sprintf('SELECT r.trashedAt FROM %s r WHERE r.cloudLibrary = :library AND r.id = :id', $class))
            ->setParameter('library', $this->library->getId(), 'uuid')->setParameter('id', $id)->setMaxResults(1)->getOneOrNullResult();

        return \is_array($row) && \is_string($row['trashedAt']) ? ValidOperationValidator::timestampMillis($row['trashedAt']) : null;
    }

    /** A work is in the trash once all its editions are, since the last of them went (D105). */
    private function workTrashed(string $work): ?int
    {
        $row = $this->entityManager->createQuery(\sprintf(
            'SELECT COUNT(e.key) AS editions, COUNT(e.trashedAt) AS trashed, MAX(e.trashedAt) AS latest
               FROM %s e JOIN e.work w WHERE e.cloudLibrary = :library AND w.cloudLibrary = :library AND w.id = :work',
            Edition::class,
        ))->setParameter('library', $this->library->getId(), 'uuid')->setParameter('work', $work)->getSingleResult();
        $count = static fn (mixed $value): int => is_numeric($value) ? (int) $value : 0;
        if (!\is_array($row) || 0 === $count($row['editions']) || $count($row['editions']) !== $count($row['trashed']) || !\is_string($row['latest'])) {
            return null;
        }

        return ValidOperationValidator::timestampMillis($row['latest']);
    }

    /** The app id of the row `$id` of `$entity` points to through `$association`, if both exist. */
    private function linked(CatalogEntity $entity, string $id, string $association): ?string
    {
        $row = $this->entityManager->createQuery(\sprintf(
            'SELECT x.id FROM %s r JOIN r.%s x WHERE r.cloudLibrary = :library AND r.id = :id',
            $entity->entityClass(),
            $association,
        ))->setParameter('library', $this->library->getId(), 'uuid')->setParameter('id', $id)->setMaxResults(1)->getOneOrNullResult();

        return \is_array($row) && \is_string($row['id']) ? $row['id'] : null;
    }

    /** The oldest of two moments in the trash: a row that hangs from two things goes with whichever went first. */
    private function oldest(?int $one, ?int $other): ?int
    {
        return null === $one || null === $other ? $one ?? $other : min($one, $other);
    }

    /** The same device, or two devices of the same person. */
    private function samePerson(string $one, string $other): bool
    {
        if ($one === $other) {
            return true;
        }
        $member = $this->memberOfDevice($one);

        return null !== $member && $member === $this->memberOfDevice($other);
    }

    private function exists(string $dql): bool
    {
        return [] !== $this->entityManager->createQuery($dql)->setParameter('library', $this->library->getId(), 'uuid')->setMaxResults(1)->getResult();
    }
}
