<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_boardgame_details')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_boardgame_details_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_boardgame_details_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_boardgame_details_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_catalog_boardgame_details_edition_key', columns: ['edition_key'])]
final class BoardgameDetails extends CatalogRow
{
    #[ORM\Column(name: 'players_min', type: Types::BIGINT, nullable: true)]
    private ?int $playersMin = null;

    #[ORM\Column(name: 'players_max', type: Types::BIGINT, nullable: true)]
    private ?int $playersMax = null;

    #[ORM\Column(name: 'duration_minutes', type: Types::BIGINT, nullable: true)]
    private ?int $durationMinutes = null;

    #[ORM\Column(name: 'min_age', type: Types::BIGINT, nullable: true)]
    private ?int $minAge = null;

    #[ORM\ManyToOne(targetEntity: Edition::class)]
    #[ORM\JoinColumn(name: 'edition_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_boardgame_details_edition_key')]
    private Edition $edition;

    public function getPlayersMin(): ?int
    {
        return $this->playersMin;
    }

    public function setPlayersMin(?int $playersMin): void
    {
        $this->playersMin = $playersMin;
    }

    public function getPlayersMax(): ?int
    {
        return $this->playersMax;
    }

    public function setPlayersMax(?int $playersMax): void
    {
        $this->playersMax = $playersMax;
    }

    public function getDurationMinutes(): ?int
    {
        return $this->durationMinutes;
    }

    public function setDurationMinutes(?int $durationMinutes): void
    {
        $this->durationMinutes = $durationMinutes;
    }

    public function getMinAge(): ?int
    {
        return $this->minAge;
    }

    public function setMinAge(?int $minAge): void
    {
        $this->minAge = $minAge;
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
