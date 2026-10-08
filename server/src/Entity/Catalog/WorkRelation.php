<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_work_relations')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_work_relations_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_work_relations_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_work_relations_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_catalog_work_relations_work_key', columns: ['work_key'])]
#[ORM\Index(name: 'idx_catalog_work_relations_related_work_key', columns: ['related_work_key'])]
final class WorkRelation extends CatalogRow
{
    #[ORM\ManyToOne(targetEntity: Work::class)]
    #[ORM\JoinColumn(name: 'work_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_work_relations_work_key')]
    private Work $work;

    #[ORM\ManyToOne(targetEntity: Work::class)]
    #[ORM\JoinColumn(name: 'related_work_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_work_relations_related_work_key')]
    private Work $relatedWork;

    #[ORM\Column(name: 'relation', type: Types::TEXT)]
    private string $relation;

    public function getWork(): Work
    {
        return $this->work;
    }

    public function setWork(Work $work): void
    {
        $this->work = $work;
    }

    public function getRelatedWork(): Work
    {
        return $this->relatedWork;
    }

    public function setRelatedWork(Work $relatedWork): void
    {
        $this->relatedWork = $relatedWork;
    }

    public function getRelation(): string
    {
        return $this->relation;
    }

    public function setRelation(string $relation): void
    {
        $this->relation = $relation;
    }
}
