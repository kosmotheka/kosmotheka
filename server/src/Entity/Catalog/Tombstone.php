<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use App\Entity\CloudLibrary;
use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

/**
 * Marks a deleted row so it cannot come back. It outlives the row, so it has no foreign key to it.
 * `entity` is the CatalogEntity value ('work'), not the app's table name.
 */
#[ORM\Entity]
#[ORM\Table(name: 'catalog_tombstones')]
#[ORM\Index(name: 'idx_catalog_tombstones_cloud_library', columns: ['cloud_library_id'])]
final class Tombstone
{
    public function __construct(
        #[ORM\Id]
        #[ORM\ManyToOne(targetEntity: CloudLibrary::class)]
        #[ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_tombstones_cloud_library')]
        private readonly CloudLibrary $cloudLibrary,
        #[ORM\Id]
        #[ORM\Column(name: 'entity', type: Types::TEXT)]
        private readonly string $entity,
        #[ORM\Id]
        #[ORM\Column(name: 'entity_id', type: Types::TEXT)]
        private readonly string $entityId,
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

    public function getHlc(): string
    {
        return $this->hlc;
    }

    public function setHlc(string $hlc): void
    {
        $this->hlc = $hlc;
    }
}
