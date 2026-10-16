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
        private bool $founded = true,
        private ?Role $role = null,
        private bool $samePerson = false,
        private bool $fenced = false,
        private bool $cloud = false,
    ) {
    }

    public function founded(): bool
    {
        return $this->founded;
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

    public function deviceHasMemberId(string $device): bool
    {
        return $this->role instanceof Role;
    }

    public function memberAccessWithdrawn(string $member): bool
    {
        return false;
    }

    public function markMember(string $id): string
    {
        return $this->samePerson ? self::MY_MEMBER : self::OTHER_MEMBER;
    }

    public function fenced(OperationPayload $operation): bool
    {
        return $this->fenced;
    }

    /** The cloud device, when there is one, is the device that makes the operation (`PermissionsTest::ME`). */
    public function cloudDevice(): ?string
    {
        return $this->cloud ? '01997c7e-0000-7000-8000-0000000000fb' : null;
    }

    public function memberExists(string $member): bool
    {
        return true;
    }

    public function memberHasAccount(string $member): bool
    {
        return self::MY_MEMBER === $member;
    }
}
