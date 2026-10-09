<?php

declare(strict_types=1);

namespace App\Entity;

use App\Repository\CloudLibraryRepository;
use Doctrine\ORM\Mapping as ORM;
use Symfony\Component\Uid\Uuid;

/**
 * A library in the cloud (D84). Its id is the server's own key for the library
 * (`C`): everything in PostgreSQL hangs from it, and nothing the client sends
 * (the local library id included) is ever used to look it up. The local id is
 * only compared, and it is set once, by the operation that links the two.
 */
#[ORM\Entity(repositoryClass: CloudLibraryRepository::class)]
#[ORM\Table(name: 'cloud_library')]
#[ORM\Index(name: 'idx_cloud_library_owner', columns: ['owner_id'])]
final class CloudLibrary
{
    #[ORM\Id]
    #[ORM\Column(type: 'uuid', unique: true)]
    private Uuid $id;

    #[ORM\Column(length: 36, nullable: true)]
    private ?string $localLibraryId = null;

    /** The operation that links the library (D93): the one CloudLink checked when it linked it, whether or not it is stored yet. */
    #[ORM\Column(type: 'uuid', nullable: true)]
    private ?Uuid $linkOperationId = null;

    #[ORM\Column]
    private \DateTimeImmutable $createdAt;

    public function __construct(
        #[ORM\ManyToOne(targetEntity: User::class)]
        #[ORM\JoinColumn(name: 'owner_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_cloud_library_owner')]
        private readonly User $owner,
    ) {
        $this->id = Uuid::v7();
        $this->createdAt = new \DateTimeImmutable('now', new \DateTimeZone('UTC'));
    }

    public function getId(): Uuid
    {
        return $this->id;
    }

    public function getOwner(): User
    {
        return $this->owner;
    }

    public function getLocalLibraryId(): ?string
    {
        return $this->localLibraryId;
    }

    public function getLinkOperationId(): ?Uuid
    {
        return $this->linkOperationId;
    }

    /** Remembers which operation links the library (the one CloudLink checked); the first one stays. */
    public function noteLinkOperation(Uuid $operationId): void
    {
        $this->linkOperationId ??= $operationId;
    }

    public function isLinked(): bool
    {
        return null !== $this->localLibraryId;
    }

    /**
     * @throws \LogicException when already linked: a cloud library keeps its local id for good
     */
    public function link(string $localLibraryId): void
    {
        if (null !== $this->localLibraryId) {
            throw new \LogicException('This cloud library is already linked.');
        }

        $this->localLibraryId = strtolower($localLibraryId);
    }

    public function getCreatedAt(): \DateTimeImmutable
    {
        return $this->createdAt;
    }
}
