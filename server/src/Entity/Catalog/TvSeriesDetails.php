<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_tv_series_details')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_tv_series_details_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_tv_series_details_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_tv_series_details_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_catalog_tv_series_details_edition_key', columns: ['edition_key'])]
final class TvSeriesDetails extends CatalogRow
{
    #[ORM\Column(name: 'seasons', type: Types::BIGINT, nullable: true)]
    private ?int $seasons = null;

    #[ORM\ManyToOne(targetEntity: Edition::class)]
    #[ORM\JoinColumn(name: 'edition_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_tv_series_details_edition_key')]
    private Edition $edition;

    public function getSeasons(): ?int
    {
        return $this->seasons;
    }

    public function setSeasons(?int $seasons): void
    {
        $this->seasons = $seasons;
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
