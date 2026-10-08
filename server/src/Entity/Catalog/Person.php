<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_people')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_people_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_people_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_people_cloud_library', columns: ['cloud_library_id'])]
final class Person extends CatalogRow
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
