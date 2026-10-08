<?php

declare(strict_types=1);

namespace App\Entity\Catalog;

use App\Entity\CloudLibrary;
use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;
use Symfony\Component\Uid\Uuid;

/**
 * A row of the catalog (D92). `key` is the server's own and never leaves it;
 * `id` is the one the app gave the row, unique only inside its cloud library.
 */
#[ORM\MappedSuperclass]
abstract class CatalogRow
{
    #[ORM\Id]
    #[ORM\Column(name: 'key', type: 'uuid')]
    private Uuid $key;

    public function __construct(
        #[ORM\ManyToOne(targetEntity: CloudLibrary::class)]
        #[ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_catalog_row_cloud_library')]
        private readonly CloudLibrary $cloudLibrary,
        #[ORM\Column(name: 'id', type: Types::TEXT)]
        private readonly string $id,
    ) {
        $this->key = Uuid::v7();
    }

    public function getKey(): Uuid
    {
        return $this->key;
    }

    public function getCloudLibrary(): CloudLibrary
    {
        return $this->cloudLibrary;
    }

    public function getId(): string
    {
        return $this->id;
    }
}
