<?php

declare(strict_types=1);

namespace App\Entity;

use App\Repository\CloudOperationRepository;
use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;
use Symfony\Component\Uid\Uuid;

/**
 * One operation in a cloud library's log, exactly as received (signature
 * included, so any device can verify it again).
 *
 * The key is `(cloud_library_id, operation_id)`, never the operation id alone
 * (D84, seam 1): operation ids are public, and a global key would let one
 * account keep another's operations out by uploading them first.
 *
 * CloudOperationRepository writes and reads this table with SQL, not through
 * the ORM: a push inserts with ON CONFLICT DO NOTHING so two devices uploading
 * the same operation at once cannot fail each other, and the body must come
 * back as it went in. Doctrine's JSON type would decode `{}` to `[]`, which
 * changes what the signature covers. Do not hydrate this entity.
 */
#[ORM\Entity(repositoryClass: CloudOperationRepository::class)]
#[ORM\Table(name: 'cloud_operation')]
#[ORM\Index(name: 'idx_cloud_operation_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_cloud_operation_author_hlc', columns: ['cloud_library_id', 'author_device_id', 'hlc'])]
final readonly class CloudOperation
{
    public function __construct(
        #[ORM\Id]
        #[ORM\ManyToOne(targetEntity: CloudLibrary::class)]
        #[ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_cloud_operation_cloud_library')]
        private CloudLibrary $cloudLibrary,
        #[ORM\Id]
        #[ORM\Column(type: 'uuid')]
        private Uuid $operationId,
        /** The device in the HLC. */
        #[ORM\Column(length: 36)]
        private string $authorDeviceId,
        /** Fixed width and lowercase hex, so byte order (collation C) is HLC order. */
        #[ORM\Column(length: 255, options: ['collation' => 'C'])]
        private string $hlc,
        #[ORM\Column(type: Types::INTEGER)]
        private int $schemaVersion,
        #[ORM\Column(type: Types::JSON, options: ['jsonb' => true])]
        private \stdClass $body,
        #[ORM\Column]
        private \DateTimeImmutable $receivedAt,
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

    public function getAuthorDeviceId(): string
    {
        return $this->authorDeviceId;
    }

    public function getHlc(): string
    {
        return $this->hlc;
    }

    public function getSchemaVersion(): int
    {
        return $this->schemaVersion;
    }

    public function getBody(): \stdClass
    {
        return $this->body;
    }

    public function getReceivedAt(): \DateTimeImmutable
    {
        return $this->receivedAt;
    }
}
