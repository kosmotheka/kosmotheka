<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use App\Entity\CloudLibrary;
use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;
use Symfony\Component\Uid\Uuid;

/**
 * An operation a fence still counts: `through` and the operations it chains to by `previous`. Worked out when
 * the fence is noted, so judging each operation of a long log never walks the chain again.
 */
#[ORM\Entity]
#[ORM\Table(name: 'catalog_device_fence_chains')]
#[ORM\Index(name: 'idx_catalog_device_fence_chains_cloud_library', columns: ['cloud_library_id'])]
final readonly class FenceChain
{
    public function __construct(
        #[ORM\Id]
        #[ORM\ManyToOne(targetEntity: CloudLibrary::class)]
        #[ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_device_fence_chains_cloud_library')]
        private CloudLibrary $cloudLibrary,
        #[ORM\Id]
        #[ORM\Column(name: 'fence_operation_id', type: 'uuid')]
        private Uuid $fenceOperationId,
        #[ORM\Id]
        #[ORM\Column(name: 'device_id', type: Types::TEXT)]
        private string $deviceId,
        #[ORM\Id]
        #[ORM\Column(name: 'operation_id', type: 'uuid')]
        private Uuid $operationId,
    ) {
    }

    public function getCloudLibrary(): CloudLibrary
    {
        return $this->cloudLibrary;
    }

    public function getFenceOperationId(): Uuid
    {
        return $this->fenceOperationId;
    }

    public function getDeviceId(): string
    {
        return $this->deviceId;
    }

    public function getOperationId(): Uuid
    {
        return $this->operationId;
    }
}
