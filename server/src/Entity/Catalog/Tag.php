<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_tags')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_tags_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_tags_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_tags_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_catalog_tags_trashed_by_key', columns: ['trashed_by_key'])]
final class Tag extends CatalogRow
{
    /** In the trash since then (D105), written as the operation wrote it; null when it is not. */
    #[ORM\Column(name: 'trashed_at', type: Types::TEXT, nullable: true)]
    private ?string $trashedAt = null;

    /** Who sent it to the trash; null when it is not in it, or when that person was deleted later. */
    #[ORM\ManyToOne(targetEntity: Member::class)]
    #[ORM\JoinColumn(name: 'trashed_by_key', referencedColumnName: 'key', nullable: true, onDelete: 'SET NULL', foreignKeyName: 'fk_catalog_tags_trashed_by_key')]
    private ?Member $trashedBy = null;

    #[ORM\Column(name: 'name', type: Types::TEXT)]
    private string $name;

    public function getName(): string
    {
        return $this->name;
    }

    public function setName(string $name): void
    {
        $this->name = $name;
    }

    public function getTrashedAt(): ?string
    {
        return $this->trashedAt;
    }

    public function setTrashedAt(?string $trashedAt): void
    {
        $this->trashedAt = $trashedAt;
    }

    public function getTrashedBy(): ?Member
    {
        return $this->trashedBy;
    }

    public function setTrashedBy(?Member $trashedBy): void
    {
        $this->trashedBy = $trashedBy;
    }
}
