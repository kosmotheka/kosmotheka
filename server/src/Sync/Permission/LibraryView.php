<?php

declare(strict_types=1);

namespace App\Sync\Permission;

use App\Dto\OperationPayload;
use App\Enum\CatalogEntity;
use App\Enum\Role;

/**
 * What the rules need to know about the library at the moment of an operation (port of `LibraryView` in
 * crates/domain/src/permission.rs). The caller answers from the library as it stood when the operation was
 * made, so every device and the server reach the same verdict.
 */
interface LibraryView
{
    /** Whether any member has a role. Until then, everyone may do everything. */
    public function hasRoles(): bool;

    /** The role of the person `$device` belongs to. Null when the device belongs to nobody or its person has no role. */
    public function roleOfDevice(string $device): ?Role;

    /** Whether the row was created by a device of the same person as `$device` (D45). */
    public function createdBySamePerson(CatalogEntity $entity, string $id, string $device): bool;

    /** Whether the operation `$request` was written by a device of the same person as `$device`: who asked may withdraw what they asked for. */
    public function requestedBySamePerson(string $request, string $device): bool;

    /**
     * The person (member id) `$device` belongs to, if any. A device whose `vouched_key` is set and is not the key
     * it is registered with belongs to nobody (D96).
     */
    public function memberOfDevice(string $device): ?string;

    /**
     * Whether the `member_id` of `$device` is set, whatever its `vouched_key` says. The cloud's rule asks this, not
     * `memberOfDevice`: it only links devices of nobody (D76).
     */
    public function deviceHasMemberId(string $device): bool;

    /** Whose the mark `$id` is, if it exists (D68). */
    public function markMember(string $id): ?string;

    /**
     * Whether `$operation` falls under a honoured fence of its author (D84, D87): dated before the operation
     * that took the author's power away, and neither the last one the administrator had nor one of its
     * ancestors through `previous`. Such an operation cannot be honest.
     * Also after a revocation of the author still in force at its HLC (D71): a revoked device writes
     * nothing, even through a device that has not heard of it yet.
     */
    public function fenced(OperationPayload $operation): bool;

    /** The device `library.cloud_device_id` names now (D76, D95), if any. */
    public function cloudDevice(): ?string;

    /** Whether the person `$member` exists. */
    public function memberExists(string $member): bool;

    /** Whether the person `$member` has an `account_id`. */
    public function memberHasAccount(string $member): bool;
}
