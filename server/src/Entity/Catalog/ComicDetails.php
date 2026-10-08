<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_comic_details')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_comic_details_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_comic_details_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_comic_details_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_catalog_comic_details_edition_key', columns: ['edition_key'])]
final class ComicDetails extends CatalogRow
{
    #[ORM\Column(name: 'number', type: Types::TEXT, nullable: true)]
    private ?string $number = null;

    #[ORM\ManyToOne(targetEntity: Edition::class)]
    #[ORM\JoinColumn(name: 'edition_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_comic_details_edition_key')]
    private Edition $edition;

    public function getNumber(): ?string
    {
        return $this->number;
    }

    public function setNumber(?string $number): void
    {
        $this->number = $number;
    }

    public function getEdition(): Edition
    {
        return $this->edition;
    }

    public function setEdition(Edition $edition): void
    {
        $this->edition = $edition;
    }
}
