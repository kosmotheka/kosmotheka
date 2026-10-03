<?php

declare(strict_types=1);

namespace App\Entity;

use App\Repository\CloudDeviceKeyRepository;
use Doctrine\ORM\Mapping as ORM;

/**
 * The key a device signs with **inside one cloud library's log** (D84, seam 4),
 * taken from the `device` create change that registered it. Never a global
 * table by device id: device ids are public, so another account could register
 * the same id with its own key in its own library, and that must not matter here.
 */
#[ORM\Entity(repositoryClass: CloudDeviceKeyRepository::class)]
#[ORM\Table(name: 'cloud_device_key')]
#[ORM\Index(name: 'idx_cloud_device_key_cloud_library', columns: ['cloud_library_id'])]
final readonly class CloudDeviceKey
{
    public function __construct(
        #[ORM\Id]
        #[ORM\ManyToOne(targetEntity: CloudLibrary::class)]
        #[ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_cloud_device_key_cloud_library')]
        private CloudLibrary $cloudLibrary,
        #[ORM\Id]
        #[ORM\Column(length: 36)]
        private string $deviceId,
        /** Ed25519, base64url without padding. */
        #[ORM\Column(length: 43)]
        private string $publicKey,
    ) {
    }

    public function getCloudLibrary(): CloudLibrary
    {
        return $this->cloudLibrary;
    }

    public function getDeviceId(): string
    {
        return $this->deviceId;
    }

    public function getPublicKey(): string
    {
        return $this->publicKey;
    }
}
