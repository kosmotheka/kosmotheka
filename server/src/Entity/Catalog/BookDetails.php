<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_book_details')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_book_details_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_book_details_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_book_details_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_catalog_book_details_edition_key', columns: ['edition_key'])]
final class BookDetails extends CatalogRow
{
    #[ORM\Column(name: 'isbn13', type: Types::TEXT, nullable: true)]
    private ?string $isbn13 = null;

    #[ORM\Column(name: 'google_books_id', type: Types::TEXT, nullable: true)]
    private ?string $googleBooksId = null;

    #[ORM\Column(name: 'open_library_id', type: Types::TEXT, nullable: true)]
    private ?string $openLibraryId = null;

    #[ORM\ManyToOne(targetEntity: Edition::class)]
    #[ORM\JoinColumn(name: 'edition_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_book_details_edition_key')]
    private Edition $edition;

    public function getIsbn13(): ?string
    {
        return $this->isbn13;
    }

    public function setIsbn13(?string $isbn13): void
    {
        $this->isbn13 = $isbn13;
    }

    public function getGoogleBooksId(): ?string
    {
        return $this->googleBooksId;
    }

    public function setGoogleBooksId(?string $googleBooksId): void
    {
        $this->googleBooksId = $googleBooksId;
    }

    public function getOpenLibraryId(): ?string
    {
        return $this->openLibraryId;
    }

    public function setOpenLibraryId(?string $openLibraryId): void
    {
        $this->openLibraryId = $openLibraryId;
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
