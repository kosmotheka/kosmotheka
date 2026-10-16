<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

/**
 * The library's own row (D84): its name and the device that is the cloud in it (D76, D95). `cloud_library_id` and
 * `cloud_server` have no column, the link is the cloud library itself; `cloud_device_id` does, because the judge needs it.
 */
#[ORM\Entity]
#[ORM\Table(name: 'catalog_libraries')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_libraries_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_libraries_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_libraries_cloud_library', columns: ['cloud_library_id'])]
final class Library extends CatalogRow
{
    #[ORM\Column(name: 'name', type: Types::TEXT)]
    private string $name;

    #[ORM\Column(name: 'cloud_device_id', type: Types::TEXT, nullable: true)]
    private ?string $cloudDeviceId = null;

    /** The nonce the id of the library derives from, with the founder's key (D97). Written by the founding, never changed. */
    #[ORM\Column(name: 'founding_nonce', type: Types::TEXT, nullable: true)]
    private ?string $foundingNonce = null;

    public function getName(): string
    {
        return $this->name;
    }

    public function setName(string $name): void
    {
        $this->name = $name;
    }

    public function getFoundingNonce(): ?string
    {
        return $this->foundingNonce;
    }

    public function setFoundingNonce(?string $foundingNonce): void
    {
        $this->foundingNonce = $foundingNonce;
    }

    public function getCloudDeviceId(): ?string
    {
        return $this->cloudDeviceId;
    }

    public function setCloudDeviceId(?string $cloudDeviceId): void
    {
        $this->cloudDeviceId = $cloudDeviceId;
    }
}
