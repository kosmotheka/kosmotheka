<?php

declare(strict_types=1);

namespace App\Tests\Support;

use App\Dto\OperationPayload;
use App\Enum\CatalogEntity;
use App\Enum\Role;
use App\Sync\Permission\LibraryView;

/** A view that answers every question the same way, whoever or whatever is asked about. */
final readonly class FakeView implements LibraryView
{
    private const string MY_MEMBER = '01997c7e-0000-7000-8000-000000000013';
    private const string OTHER_MEMBER = '01997c7e-0000-7000-8000-000000000012';

    public function __construct(
        private bool $roles = true,
        private ?Role $role = null,
        private bool $samePerson = false,
        private bool $fenced = false,
    ) {
    }

    public function hasRoles(): bool
    {
        return $this->roles;
    }

    public function roleOfDevice(string $device): ?Role
    {
        return $this->role;
    }

    public function createdBySamePerson(CatalogEntity $entity, string $id, string $device): bool
    {
        return $this->samePerson;
    }

    public function requestedBySamePerson(string $request, string $device): bool
    {
        return $this->samePerson;
    }

    public function memberOfDevice(string $device): ?string
    {
        return $this->role instanceof Role ? self::MY_MEMBER : null;
    }

    public function markMember(string $id): string
    {
        return $this->samePerson ? self::MY_MEMBER : self::OTHER_MEMBER;
    }

    public function fenced(OperationPayload $operation): bool
    {
        return $this->fenced;
    }
}
