<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_work_genres')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_work_genres_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_work_genres_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_work_genres_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_catalog_work_genres_work_key', columns: ['work_key'])]
final class WorkGenre extends CatalogRow
{
    #[ORM\ManyToOne(targetEntity: Work::class)]
    #[ORM\JoinColumn(name: 'work_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_work_genres_work_key')]
    private Work $work;

    #[ORM\Column(name: 'genre', type: Types::TEXT)]
    private string $genre;

    public function getWork(): Work
    {
        return $this->work;
    }

    public function setWork(Work $work): void
    {
        $this->work = $work;
    }

    public function getGenre(): string
    {
        return $this->genre;
    }

    public function setGenre(string $genre): void
    {
        $this->genre = $genre;
    }
}
