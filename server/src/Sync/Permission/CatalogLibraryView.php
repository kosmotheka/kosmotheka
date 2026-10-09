<?php

declare(strict_types=1);

namespace App\Sync\Permission;

use App\Dto\OperationPayload;
use App\Entity\Catalog\Device;
use App\Entity\Catalog\Fence;
use App\Entity\Catalog\FenceChain;
use App\Entity\Catalog\Mark;
use App\Entity\Catalog\Member;
use App\Entity\Catalog\Revocation;
use App\Entity\Catalog\RowAuthor;
use App\Entity\CloudLibrary;
use App\Entity\CloudOperation;
use App\Enum\CatalogEntity;
use App\Enum\Role;
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
    public function __construct(
        private EntityManagerInterface $entityManager,
        private CloudLibrary $library,
    ) {
    }

    public function hasRoles(): bool
    {
        return $this->exists(\sprintf('SELECT 1 FROM %s m WHERE m.cloudLibrary = :library AND m.role IS NOT NULL', Member::class));
    }

    /**
     * Whether operations of the library must be judged again when something arrives late: roles, fences
     * or revocations depend on the order of the log (D73, D84).
     */
    public function judgedByOrder(): bool
    {
        return $this->hasRoles()
            || $this->exists(\sprintf('SELECT 1 FROM %s f WHERE f.cloudLibrary = :library', Fence::class))
            || $this->exists(\sprintf('SELECT 1 FROM %s r WHERE r.cloudLibrary = :library', Revocation::class));
    }

    public function roleOfDevice(string $device): ?Role
    {
        $role = $this->entityManager->createQuery(\sprintf(
            'SELECT m.role FROM %s d JOIN d.member m WHERE d.cloudLibrary = :library AND m.cloudLibrary = :library AND d.id = :device',
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
            'SELECT m.id FROM %s d JOIN d.member m WHERE d.cloudLibrary = :library AND m.cloudLibrary = :library AND d.id = :device',
            Device::class,
        ))->setParameter('library', $this->library->getId(), 'uuid')->setParameter('device', $device)->setMaxResults(1)->getOneOrNullResult();

        return \is_array($member) && \is_string($member['id']) ? $member['id'] : null;
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
        $device = Permissions::author($operation);

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
