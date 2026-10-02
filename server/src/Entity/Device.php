<?php

declare(strict_types=1);

namespace App\Entity;

use App\Repository\DeviceRepository;
use Doctrine\ORM\Mapping as ORM;

/**
 * One app install (milestone 7.1, part 2): its id is the device id the Rust
 * app already uses in its HLCs, not generated here. `tokenHash` null means
 * signed out; `removedAt` means removed for good (D81): the install was handed
 * to someone else and must start again with a new identity. A device id is
 * bound to its account forever, checked by App\Controller\AuthController
 * before a row is created.
 */
#[ORM\Entity(repositoryClass: DeviceRepository::class)]
#[ORM\Table(name: 'device')]
#[ORM\Index(name: 'idx_device_user', columns: ['user_id'])]
final class Device
{
    #[ORM\Column]
    private \DateTimeImmutable $createdAt;

    #[ORM\Column(nullable: true)]
    private ?\DateTimeImmutable $lastSeenAt = null;

    #[ORM\Column(nullable: true)]
    private ?\DateTimeImmutable $removedAt = null;

    public function __construct(
        #[ORM\Id]
        #[ORM\Column(length: 36)]
        private readonly string $id,
        #[ORM\ManyToOne(targetEntity: User::class)]
        #[ORM\JoinColumn(name: 'user_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_device_user')]
        private User $user,
        #[ORM\Column(length: 100)]
        private string $name,
        /** Ed25519, base64url without padding (App\Sync\OperationSignature decodes the same format). */
        #[ORM\Column(length: 43)]
        private readonly string $publicKey,
        /** SHA-256 hex of the current token; null means signed out. */
        #[ORM\Column(length: 64, nullable: true)]
        private ?string $tokenHash = null,
    ) {
        $this->createdAt = new \DateTimeImmutable('now', new \DateTimeZone('UTC'));
    }

    public function getId(): string
    {
        return $this->id;
    }

    public function getUser(): User
    {
        return $this->user;
    }

    public function getName(): string
    {
        return $this->name;
    }

    public function setName(string $name): void
    {
        $this->name = $name;
    }

    public function getPublicKey(): string
    {
        return $this->publicKey;
    }

    public function getTokenHash(): ?string
    {
        return $this->tokenHash;
    }

    public function setTokenHash(?string $tokenHash): void
    {
        $this->tokenHash = $tokenHash;
    }

    public function isSignedIn(): bool
    {
        return null !== $this->tokenHash;
    }

    public function getCreatedAt(): \DateTimeImmutable
    {
        return $this->createdAt;
    }

    public function getLastSeenAt(): ?\DateTimeImmutable
    {
        return $this->lastSeenAt;
    }

    public function touch(\DateTimeImmutable $at): void
    {
        $this->lastSeenAt = $at;
    }

    public function isRemoved(): bool
    {
        return $this->removedAt instanceof \DateTimeImmutable;
    }

    /** Signs the device out for good (D81): it can never sign in again. */
    public function remove(\DateTimeImmutable $at): void
    {
        $this->tokenHash = null;
        $this->removedAt ??= $at;
    }
}
