<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use App\Entity\CloudLibrary;
use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

/**
 * Version of one field of one row: the HLC of the operation that last wrote it (the merge compares it, byte by byte).
 * `entity` is the CatalogEntity value ('work'), not the app's table name.
 */
#[ORM\Entity]
#[ORM\Table(name: 'catalog_field_versions')]
#[ORM\Index(name: 'idx_catalog_field_versions_cloud_library', columns: ['cloud_library_id'])]
final class FieldVersion
{
    public function __construct(
        #[ORM\Id]
        #[ORM\ManyToOne(targetEntity: CloudLibrary::class)]
        #[ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_field_versions_cloud_library')]
        private readonly CloudLibrary $cloudLibrary,
        #[ORM\Id]
        #[ORM\Column(name: 'entity', type: Types::TEXT)]
        private readonly string $entity,
        #[ORM\Id]
        #[ORM\Column(name: 'entity_id', type: Types::TEXT)]
        private readonly string $entityId,
        #[ORM\Id]
        #[ORM\Column(name: 'field', type: Types::TEXT)]
        private readonly string $field,
        #[ORM\Column(name: 'hlc', type: Types::TEXT, options: ['collation' => 'C'])]
        private string $hlc,
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

    public function getField(): string
    {
        return $this->field;
    }

    public function getHlc(): string
    {
        return $this->hlc;
    }

    public function setHlc(string $hlc): void
    {
        $this->hlc = $hlc;
    }
}
