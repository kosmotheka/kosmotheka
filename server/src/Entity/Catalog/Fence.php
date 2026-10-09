<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use App\Entity\CloudLibrary;
use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;
use Symfony\Component\Uid\Uuid;

/**
 * A fence set by an applied operation (D84, D87): the operation `operationId` took the power of `deviceId`
 * away, knowing its operations only up to `through`. What that device wrote before `untilHlc` and is not
 * `through` or one of its ancestors (see FenceChain) cannot be honest.
 */
#[ORM\Entity]
#[ORM\Table(name: 'catalog_device_fences')]
#[ORM\Index(name: 'idx_catalog_device_fences_cloud_library', columns: ['cloud_library_id'])]
final readonly class Fence
{
    public function __construct(
        #[ORM\Id]
        #[ORM\ManyToOne(targetEntity: CloudLibrary::class)]
        #[ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_device_fences_cloud_library')]
        private CloudLibrary $cloudLibrary,
        #[ORM\Id]
        #[ORM\Column(name: 'operation_id', type: 'uuid')]
        private Uuid $operationId,
        #[ORM\Id]
        #[ORM\Column(name: 'device_id', type: Types::TEXT)]
        private string $deviceId,
        /** The id of the last operation of the device the fence honours; empty when it knew none. */
        #[ORM\Column(name: 'through', type: Types::TEXT)]
        private string $through,
        /** The HLC of the operation that sets the fence. */
        #[ORM\Column(name: 'until_hlc', type: Types::TEXT, options: ['collation' => 'C'])]
        private string $untilHlc,
    ) {
    }

    public function getCloudLibrary(): CloudLibrary
    {
        return $this->cloudLibrary;
    }

    public function getOperationId(): Uuid
    {
        return $this->operationId;
    }

    public function getDeviceId(): string
    {
        return $this->deviceId;
    }

    public function getThrough(): string
    {
        return $this->through;
    }

    public function getUntilHlc(): string
    {
        return $this->untilHlc;
    }
}
