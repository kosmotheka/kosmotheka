<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_marks')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_marks_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_marks_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_marks_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_catalog_marks_member_key', columns: ['member_key'])]
#[ORM\Index(name: 'idx_catalog_marks_work_key', columns: ['work_key'])]
final class Mark extends CatalogRow
{
    #[ORM\ManyToOne(targetEntity: Member::class)]
    #[ORM\JoinColumn(name: 'member_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_marks_member_key')]
    private Member $member;

    #[ORM\ManyToOne(targetEntity: Work::class)]
    #[ORM\JoinColumn(name: 'work_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_marks_work_key')]
    private Work $work;

    #[ORM\Column(name: 'status', type: Types::TEXT, nullable: true)]
    private ?string $status = null;

    #[ORM\Column(name: 'status_on', type: Types::TEXT, nullable: true)]
    private ?string $statusOn = null;

    #[ORM\Column(name: 'rating', type: Types::INTEGER, nullable: true)]
    private ?int $rating = null;

    #[ORM\Column(name: 'favorite', type: Types::BOOLEAN, nullable: true)]
    private ?bool $favorite = null;

    #[ORM\Column(name: 'wished', type: Types::BOOLEAN, nullable: true)]
    private ?bool $wished = null;

    public function getMember(): Member
    {
        return $this->member;
    }

    public function setMember(Member $member): void
    {
        $this->member = $member;
    }

    public function getWork(): Work
    {
        return $this->work;
    }

    public function setWork(Work $work): void
    {
        $this->work = $work;
    }

    public function getStatus(): ?string
    {
        return $this->status;
    }

    public function setStatus(?string $status): void
    {
        $this->status = $status;
    }

    public function getStatusOn(): ?string
    {
        return $this->statusOn;
    }

    public function setStatusOn(?string $statusOn): void
    {
        $this->statusOn = $statusOn;
    }

    public function getRating(): ?int
    {
        return $this->rating;
    }

    public function setRating(?int $rating): void
    {
        $this->rating = $rating;
    }

    public function getFavorite(): ?bool
    {
        return $this->favorite;
    }

    public function setFavorite(?bool $favorite): void
    {
        $this->favorite = $favorite;
    }

    public function getWished(): ?bool
    {
        return $this->wished;
    }

    public function setWished(?bool $wished): void
    {
        $this->wished = $wished;
    }
}
