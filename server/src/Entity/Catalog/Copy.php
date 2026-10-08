<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_copies')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_copies_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_copies_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_copies_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_catalog_copies_edition_key', columns: ['edition_key'])]
#[ORM\Index(name: 'idx_catalog_copies_owner_key', columns: ['owner_key'])]
final class Copy extends CatalogRow
{
    #[ORM\ManyToOne(targetEntity: Edition::class)]
    #[ORM\JoinColumn(name: 'edition_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_copies_edition_key')]
    private Edition $edition;

    #[ORM\ManyToOne(targetEntity: Member::class)]
    #[ORM\JoinColumn(name: 'owner_key', referencedColumnName: 'key', nullable: true, onDelete: 'SET NULL', foreignKeyName: 'fk_catalog_copies_owner_key')]
    private ?Member $owner = null;

    #[ORM\Column(name: 'format', type: Types::TEXT)]
    private string $format;

    #[ORM\Column(name: 'quantity', type: Types::BIGINT)]
    private int $quantity;

    #[ORM\Column(name: 'location', type: Types::TEXT, nullable: true)]
    private ?string $location = null;

    #[ORM\Column(name: 'acquired_on', type: Types::TEXT, nullable: true)]
    private ?string $acquiredOn = null;

    #[ORM\Column(name: 'platform', type: Types::TEXT, nullable: true)]
    private ?string $platform = null;

    #[ORM\Column(name: 'seasons', type: Types::TEXT, nullable: true)]
    private ?string $seasons = null;

    public function getEdition(): Edition
    {
        return $this->edition;
    }

    public function setEdition(Edition $edition): void
    {
        $this->edition = $edition;
    }

    public function getOwner(): ?Member
    {
        return $this->owner;
    }

    public function setOwner(?Member $owner): void
    {
        $this->owner = $owner;
    }

    public function getFormat(): string
    {
        return $this->format;
    }

    public function setFormat(string $format): void
    {
        $this->format = $format;
    }

    public function getQuantity(): int
    {
        return $this->quantity;
    }

    public function setQuantity(int $quantity): void
    {
        $this->quantity = $quantity;
    }

    public function getLocation(): ?string
    {
        return $this->location;
    }

    public function setLocation(?string $location): void
    {
        $this->location = $location;
    }

    public function getAcquiredOn(): ?string
    {
        return $this->acquiredOn;
    }

    public function setAcquiredOn(?string $acquiredOn): void
    {
        $this->acquiredOn = $acquiredOn;
    }

    public function getPlatform(): ?string
    {
        return $this->platform;
    }

    public function setPlatform(?string $platform): void
    {
        $this->platform = $platform;
    }

    public function getSeasons(): ?string
    {
        return $this->seasons;
    }

    public function setSeasons(?string $seasons): void
    {
        $this->seasons = $seasons;
    }
}
