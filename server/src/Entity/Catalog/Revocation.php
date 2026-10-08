<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_revocations')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_revocations_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_revocations_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_revocations_cloud_library', columns: ['cloud_library_id'])]
final class Revocation extends CatalogRow
{
    #[ORM\Column(name: 'device_id', type: Types::TEXT)]
    private string $deviceId;

    #[ORM\Column(name: 'device_name', type: Types::TEXT)]
    private string $deviceName;

    public function getDeviceId(): string
    {
        return $this->deviceId;
    }

    public function setDeviceId(string $deviceId): void
    {
        $this->deviceId = $deviceId;
    }

    public function getDeviceName(): string
    {
        return $this->deviceName;
    }

    public function setDeviceName(string $deviceName): void
    {
        $this->deviceName = $deviceName;
    }
}
