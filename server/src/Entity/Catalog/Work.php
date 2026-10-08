<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_works')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_works_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_works_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_works_cloud_library', columns: ['cloud_library_id'])]
final class Work extends CatalogRow
{
    #[ORM\Column(name: 'kind', type: Types::TEXT)]
    private string $kind;

    #[ORM\Column(name: 'title', type: Types::TEXT)]
    private string $title;

    public function getKind(): string
    {
        return $this->kind;
    }

    public function setKind(string $kind): void
    {
        $this->kind = $kind;
    }

    public function getTitle(): string
    {
        return $this->title;
    }

    public function setTitle(string $title): void
    {
        $this->title = $title;
    }
}
