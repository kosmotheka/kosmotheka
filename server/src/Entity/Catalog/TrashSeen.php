<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use App\Entity\CloudLibrary;
use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

/**
 * When this server first saw a row in the trash of a cloud library (D105), in milliseconds since 1970. The cloud
 * counts its 60 days from the later of this and `trashed_at`, so a device with a wrong clock does not make it delete
 * early. The server's own: never synced, rebuilt by the next run if lost.
 */
#[ORM\Entity]
#[ORM\Table(name: 'catalog_trash_seen')]
#[ORM\Index(name: 'idx_catalog_trash_seen_cloud_library', columns: ['cloud_library_id'])]
final readonly class TrashSeen
{
    public function __construct(
        #[ORM\Id]
        #[ORM\ManyToOne(targetEntity: CloudLibrary::class)]
        #[ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_trash_seen_cloud_library')]
        private CloudLibrary $cloudLibrary,
        /** The catalog entity, as operations name it (`edition`, `copy`, `tag`, `series`). */
        #[ORM\Id]
        #[ORM\Column(name: 'entity', type: Types::TEXT)]
        private string $entity,
        /** The row's id in the app (unique inside its library). */
        #[ORM\Id]
        #[ORM\Column(name: 'entity_id', type: Types::TEXT)]
        private string $entityId,
        #[ORM\Column(name: 'seen_at', type: Types::BIGINT)]
        private int $seenAt,
    ) {
    }

    public function getEntity(): string
    {
        return $this->entity;
    }

    public function getEntityId(): string
    {
        return $this->entityId;
    }

    public function getSeenAt(): int
    {
        return $this->seenAt;
    }
}
