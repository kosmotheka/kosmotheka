<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use App\Entity\CloudLibrary;
use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

/**
 * The device that created a row; the permissions read it.
 * `entity` is the CatalogEntity value ('work'), not the app's table name.
 */
#[ORM\Entity]
#[ORM\Table(name: 'catalog_row_authors')]
#[ORM\Index(name: 'idx_catalog_row_authors_cloud_library', columns: ['cloud_library_id'])]
final readonly class RowAuthor
{
    public function __construct(
        #[ORM\Id]
        #[ORM\ManyToOne(targetEntity: CloudLibrary::class)]
        #[ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_row_authors_cloud_library')]
        private CloudLibrary $cloudLibrary,
        #[ORM\Id]
        #[ORM\Column(name: 'entity', type: Types::TEXT)]
        private string $entity,
        #[ORM\Id]
        #[ORM\Column(name: 'entity_id', type: Types::TEXT)]
        private string $entityId,
        #[ORM\Column(name: 'device_id', type: Types::TEXT)]
        private string $deviceId,
    ) {
    }

    public function getCloudLibrary(): CloudLibrary
    {
        return $this->cloudLibrary;
    }

    public function getEntity(): string
    {
        return $this->entity;
    }

    public function getEntityId(): string
    {
        return $this->entityId;
    }

    public function getDeviceId(): string
    {
        return $this->deviceId;
    }
}
