<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_series')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_series_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_series_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_series_cloud_library', columns: ['cloud_library_id'])]
final class Series extends CatalogRow
{
    #[ORM\Column(name: 'name', type: Types::TEXT)]
    private string $name;

    #[ORM\Column(name: 'ordered', type: Types::BOOLEAN)]
    private bool $ordered;

    public function getName(): string
    {
        return $this->name;
    }

    public function setName(string $name): void
    {
        $this->name = $name;
    }

    public function getOrdered(): bool
    {
        return $this->ordered;
    }

    public function setOrdered(bool $ordered): void
    {
        $this->ordered = $ordered;
    }
}
