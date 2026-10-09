<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use App\Entity\CloudLibrary;
use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;
use Symfony\Component\Uid\Uuid;

/**
 * A seal set by an applied operation (D94): the operation `operationId` fenced the device of its own author,
 * so it fences every device. The devices it names are the rows of Fence with the same operation; of any
 * other, nothing dated before `untilHlc` is believed.
 */
#[ORM\Entity]
#[ORM\Table(name: 'catalog_device_seals')]
#[ORM\Index(name: 'idx_catalog_device_seals_cloud_library', columns: ['cloud_library_id'])]
final readonly class Seal
{
    public function __construct(
        #[ORM\Id]
        #[ORM\ManyToOne(targetEntity: CloudLibrary::class)]
        #[ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_device_seals_cloud_library')]
        private CloudLibrary $cloudLibrary,
        #[ORM\Id]
        #[ORM\Column(name: 'operation_id', type: 'uuid')]
        private Uuid $operationId,
        /** The HLC of the operation that seals. */
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

    public function getUntilHlc(): string
    {
        return $this->untilHlc;
    }
}
