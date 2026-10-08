<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

/** The library's own row (D84): its name. `cloud_library_id` and `cloud_server` have no column, the link is the cloud library itself. */
#[ORM\Entity]
#[ORM\Table(name: 'catalog_libraries')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_libraries_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_libraries_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_libraries_cloud_library', columns: ['cloud_library_id'])]
final class Library extends CatalogRow
{
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
}
