<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_members')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_members_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_members_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_members_cloud_library', columns: ['cloud_library_id'])]
final class Member extends CatalogRow
{
    #[ORM\Column(name: 'name', type: Types::TEXT)]
    private string $name;

    #[ORM\Column(name: 'role', type: Types::TEXT, nullable: true)]
    private ?string $role = null;

    #[ORM\Column(name: 'can_share', type: Types::BOOLEAN, nullable: true)]
    private ?bool $canShare = null;

    /** The account linked to this person (D76); only the cloud device writes it. */
    #[ORM\Column(name: 'account_id', type: Types::TEXT, nullable: true)]
    private ?string $accountId = null;

    /** The person may no longer reach the library (D98); null reads as false. */
    #[ORM\Column(name: 'access_withdrawn', type: Types::BOOLEAN, nullable: true)]
    private ?bool $accessWithdrawn = null;

    public function getName(): string
    {
        return $this->name;
    }

    public function setName(string $name): void
    {
        $this->name = $name;
    }

    public function getRole(): ?string
    {
        return $this->role;
    }

    public function setRole(?string $role): void
    {
        $this->role = $role;
    }

    public function getCanShare(): ?bool
    {
        return $this->canShare;
    }

    public function setCanShare(?bool $canShare): void
    {
        $this->canShare = $canShare;
    }

    public function getAccountId(): ?string
    {
        return $this->accountId;
    }

    public function setAccountId(?string $accountId): void
    {
        $this->accountId = $accountId;
    }

    public function getAccessWithdrawn(): ?bool
    {
        return $this->accessWithdrawn;
    }

    public function setAccessWithdrawn(?bool $accessWithdrawn): void
    {
        $this->accessWithdrawn = $accessWithdrawn;
    }
}
