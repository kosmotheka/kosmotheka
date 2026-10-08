<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_work_contributors')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_work_contributors_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\UniqueConstraint(name: 'uniq_catalog_work_contributors_work_person_role', columns: ['work_key', 'person_key', 'role'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_work_contributors_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_work_contributors_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_catalog_work_contributors_work_key', columns: ['work_key'])]
#[ORM\Index(name: 'idx_catalog_work_contributors_person_key', columns: ['person_key'])]
final class WorkContributor extends CatalogRow
{
    #[ORM\ManyToOne(targetEntity: Work::class)]
    #[ORM\JoinColumn(name: 'work_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_work_contributors_work_key')]
    private Work $work;

    #[ORM\ManyToOne(targetEntity: Person::class)]
    #[ORM\JoinColumn(name: 'person_key', referencedColumnName: 'key', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_work_contributors_person_key')]
    private Person $person;

    #[ORM\Column(name: 'role', type: Types::TEXT)]
    private string $role;

    #[ORM\Column(name: 'position', type: Types::BIGINT)]
    private int $position;

    public function getWork(): Work
    {
        return $this->work;
    }

    public function setWork(Work $work): void
    {
        $this->work = $work;
    }

    public function getPerson(): Person
    {
        return $this->person;
    }

    public function setPerson(Person $person): void
    {
        $this->person = $person;
    }

    public function getRole(): string
    {
        return $this->role;
    }

    public function setRole(string $role): void
    {
        $this->role = $role;
    }

    public function getPosition(): int
    {
        return $this->position;
    }

    public function setPosition(int $position): void
    {
        $this->position = $position;
    }
}
