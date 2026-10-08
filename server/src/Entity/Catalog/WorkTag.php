<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_work_tags')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_work_tags_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_work_tags_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_work_tags_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_catalog_work_tags_work_key', columns: ['work_key'])]
#[ORM\Index(name: 'idx_catalog_work_tags_tag_key', columns: ['tag_key'])]
final class WorkTag extends CatalogRow
{
    #[ORM\ManyToOne(targetEntity: Work::class)]
    #[ORM\JoinColumn(name: 'work_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_work_tags_work_key')]
    private Work $work;

    #[ORM\ManyToOne(targetEntity: Tag::class)]
    #[ORM\JoinColumn(name: 'tag_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_work_tags_tag_key')]
    private Tag $tag;

    public function getWork(): Work
    {
        return $this->work;
    }

    public function setWork(Work $work): void
    {
        $this->work = $work;
    }

    public function getTag(): Tag
    {
        return $this->tag;
    }

    public function setTag(Tag $tag): void
    {
        $this->tag = $tag;
    }
}
