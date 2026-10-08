<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_editions')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_editions_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_editions_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_editions_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_catalog_editions_work_key', columns: ['work_key'])]
#[ORM\Index(name: 'idx_catalog_editions_publisher_key', columns: ['publisher_key'])]
final class Edition extends CatalogRow
{
    #[ORM\ManyToOne(targetEntity: Work::class)]
    #[ORM\JoinColumn(name: 'work_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_editions_work_key')]
    private Work $work;

    #[ORM\Column(name: 'title', type: Types::TEXT)]
    private string $title;

    #[ORM\Column(name: 'subtitle', type: Types::TEXT, nullable: true)]
    private ?string $subtitle = null;

    #[ORM\ManyToOne(targetEntity: Publisher::class)]
    #[ORM\JoinColumn(name: 'publisher_key', referencedColumnName: 'key', nullable: true, onDelete: 'SET NULL', foreignKeyName: 'fk_catalog_editions_publisher_key')]
    private ?Publisher $publisher = null;

    #[ORM\Column(name: 'published_year', type: Types::BIGINT, nullable: true)]
    private ?int $publishedYear = null;

    #[ORM\Column(name: 'language', type: Types::TEXT, nullable: true)]
    private ?string $language = null;

    #[ORM\Column(name: 'cover', type: Types::TEXT, nullable: true)]
    private ?string $cover = null;

    #[ORM\Column(name: 'barcode', type: Types::TEXT, nullable: true)]
    private ?string $barcode = null;

    #[ORM\Column(name: 'notes', type: Types::TEXT, nullable: true)]
    private ?string $notes = null;

    public function getWork(): Work
    {
        return $this->work;
    }

    public function setWork(Work $work): void
    {
        $this->work = $work;
    }

    public function getTitle(): string
    {
        return $this->title;
    }

    public function setTitle(string $title): void
    {
        $this->title = $title;
    }

    public function getSubtitle(): ?string
    {
        return $this->subtitle;
    }

    public function setSubtitle(?string $subtitle): void
    {
        $this->subtitle = $subtitle;
    }

    public function getPublisher(): ?Publisher
    {
        return $this->publisher;
    }

    public function setPublisher(?Publisher $publisher): void
    {
        $this->publisher = $publisher;
    }

    public function getPublishedYear(): ?int
    {
        return $this->publishedYear;
    }

    public function setPublishedYear(?int $publishedYear): void
    {
        $this->publishedYear = $publishedYear;
    }

    public function getLanguage(): ?string
    {
        return $this->language;
    }

    public function setLanguage(?string $language): void
    {
        $this->language = $language;
    }

    public function getCover(): ?string
    {
        return $this->cover;
    }

    public function setCover(?string $cover): void
    {
        $this->cover = $cover;
    }

    public function getBarcode(): ?string
    {
        return $this->barcode;
    }

    public function setBarcode(?string $barcode): void
    {
        $this->barcode = $barcode;
    }

    public function getNotes(): ?string
    {
        return $this->notes;
    }

    public function setNotes(?string $notes): void
    {
        $this->notes = $notes;
    }
}
