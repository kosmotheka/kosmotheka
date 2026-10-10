<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity]
#[ORM\Table(name: 'catalog_devices')]
#[ORM\UniqueConstraint(name: 'uniq_catalog_devices_library_id', columns: ['cloud_library_id', 'id'])]
#[ORM\AssociationOverrides([new ORM\AssociationOverride(name: 'cloudLibrary', joinColumns: [new ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_devices_cloud_library')])])]
#[ORM\Index(name: 'idx_catalog_devices_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_catalog_devices_member_key', columns: ['member_key'])]
final class Device extends CatalogRow
{
    #[ORM\Column(name: 'public_key', type: Types::TEXT)]
    private string $publicKey;

    #[ORM\Column(name: 'name', type: Types::TEXT)]
    private string $name;

    #[ORM\ManyToOne(targetEntity: Member::class)]
    #[ORM\JoinColumn(name: 'member_key', referencedColumnName: 'key', nullable: true, onDelete: 'SET NULL', foreignKeyName: 'fk_catalog_devices_member_key')]
    private ?Member $member = null;

    #[ORM\Column(name: 'trusted_through', type: Types::TEXT, nullable: true)]
    private ?string $trustedThrough = null;

    #[ORM\Column(name: 'vouched_key', type: Types::TEXT, nullable: true)]
    private ?string $vouchedKey = null;

    public function getPublicKey(): string
    {
        return $this->publicKey;
    }

    public function setPublicKey(string $publicKey): void
    {
        $this->publicKey = $publicKey;
    }

    public function getName(): string
    {
        return $this->name;
    }

    public function setName(string $name): void
    {
        $this->name = $name;
    }

    public function getMember(): ?Member
    {
        return $this->member;
    }

    public function setMember(?Member $member): void
    {
        $this->member = $member;
    }

    public function getTrustedThrough(): ?string
    {
        return $this->trustedThrough;
    }

    public function setTrustedThrough(?string $trustedThrough): void
    {
        $this->trustedThrough = $trustedThrough;
    }

    public function getVouchedKey(): ?string
    {
        return $this->vouchedKey;
    }

    public function setVouchedKey(?string $vouchedKey): void
    {
        $this->vouchedKey = $vouchedKey;
    }
}
