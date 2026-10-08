<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_series_entries')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_series_entries_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_series_entries_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_series_entries_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_catalog_series_entries_series_key', columns: ['series_key'])]
#[ORM\Index(name: 'idx_catalog_series_entries_work_key', columns: ['work_key'])]
final class SeriesEntry extends CatalogRow
{
    #[ORM\ManyToOne(targetEntity: Series::class)]
    #[ORM\JoinColumn(name: 'series_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_series_entries_series_key')]
    private Series $series;

    #[ORM\ManyToOne(targetEntity: Work::class)]
    #[ORM\JoinColumn(name: 'work_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_series_entries_work_key')]
    private Work $work;

    #[ORM\Column(name: 'position', type: Types::BIGINT, nullable: true)]
    private ?int $position = null;

    public function getSeries(): Series
    {
        return $this->series;
    }

    public function setSeries(Series $series): void
    {
        $this->series = $series;
    }

    public function getWork(): Work
    {
        return $this->work;
    }

    public function setWork(Work $work): void
    {
        $this->work = $work;
    }

    public function getPosition(): ?int
    {
        return $this->position;
    }

    public function setPosition(?int $position): void
    {
        $this->position = $position;
    }
}
