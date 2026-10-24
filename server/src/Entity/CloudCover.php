<?php

declare(strict_types=1);

namespace App\Entity;

use App\Repository\CloudCoverRepository;
use Doctrine\ORM\Mapping as ORM;
use Symfony\Component\Uid\Uuid;

/**
 * A cover file the server keeps for a cloud library (D82): the file is in `app.covers_dir`, under the library's id,
 * named `<sha256>.<ext>` as every device names it; this row is what the limit per library and the cleanup read.
 */
#[ORM\Entity(repositoryClass: CloudCoverRepository::class)]
#[ORM\Table(name: 'cloud_cover')]
#[ORM\UniqueConstraint(name: 'uniq_cloud_cover_library_name', columns: ['cloud_library_id', 'name'])]
#[ORM\Index(name: 'idx_cloud_cover_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_cloud_cover_stored_at', columns: ['stored_at'])]
class CloudCover
{
    #[ORM\Id]
    #[ORM\Column(type: 'uuid')]
    private readonly Uuid $id;

    public function __construct(
        #[ORM\ManyToOne(targetEntity: CloudLibrary::class)]
        #[ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_cloud_cover_cloud_library')]
        private readonly CloudLibrary $cloudLibrary,
        #[ORM\Column(length: 69)]
        private readonly string $name,
        #[ORM\Column]
        private readonly int $size,
        #[ORM\Column]
        private readonly \DateTimeImmutable $storedAt,
    ) {
        $this->id = Uuid::v7();
    }

    public function getCloudLibrary(): CloudLibrary
    {
        return $this->cloudLibrary;
    }

    public function getName(): string
    {
        return $this->name;
    }

    public function getSize(): int
    {
        return $this->size;
    }

    public function getStoredAt(): \DateTimeImmutable
    {
        return $this->storedAt;
    }
}
