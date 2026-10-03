<?php

declare(strict_types=1);

namespace App\Entity;

use App\Enum\SignOutReason;
use App\Repository\DeviceRepository;
use Doctrine\ORM\Mapping as ORM;
use Symfony\Component\Uid\Uuid;

/**
 * One app install (milestone 7.1, part 2). The primary key is generated here;
 * `deviceId` is the id the Rust app already uses in its HLCs. Those ids are
 * public (they travel in every operation), so the server identifies an
 * install by its id together with its key, never by the id alone (D84, SEG-016):
 * otherwise whoever signed in first with someone else's id would own it.
 * Two accounts may hold rows with the same id and different keys. `tokenHash` null means
 * signed out; `removedAt` means removed (D81): the install was handed to
 * someone else and must start again with a new identity. Until the install
 * is told so (`removalNoticedAt`), the removal can be undone and the device
 * carries on with its token, as if nothing had happened. The same id and key
 * cannot sign into a second account: App\Controller\AuthController checks it.
 */
#[ORM\Entity(repositoryClass: DeviceRepository::class)]
#[ORM\Table(name: 'device')]
#[ORM\Index(name: 'idx_device_user', columns: ['user_id'])]
#[ORM\UniqueConstraint(name: 'uniq_device_id_key', columns: ['device_id', 'public_key'])]
#[ORM\UniqueConstraint(name: 'uniq_device_user_device_id', columns: ['user_id', 'device_id'])]
#[ORM\UniqueConstraint(name: 'uniq_device_token_hash', columns: ['token_hash'])]
final class Device
{
    #[ORM\Id]
    #[ORM\Column(type: 'uuid', unique: true)]
    private Uuid $id;

    #[ORM\Column]
    private \DateTimeImmutable $createdAt;

    #[ORM\Column(nullable: true)]
    private ?\DateTimeImmutable $lastSeenAt = null;

    #[ORM\Column(nullable: true)]
    private ?\DateTimeImmutable $removedAt = null;

    /**
     * Set when the server signs the device out for a reason the person must
     * hear about. The token hash is kept so only the real install is told.
     */
    #[ORM\Column(length: 32, nullable: true, enumType: SignOutReason::class)]
    private ?SignOutReason $signOutReason = null;

    /** When the install itself was first told it was removed; from then on it has wiped its data. */
    #[ORM\Column(nullable: true)]
    private ?\DateTimeImmutable $removalNoticedAt = null;

    public function __construct(
        /** The app's own device id. */
        #[ORM\Column(length: 36)]
        private readonly string $deviceId,
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
        $this->id = Uuid::v7();
        $this->createdAt = new \DateTimeImmutable('now', new \DateTimeZone('UTC'));
    }

    public function getId(): Uuid
    {
        return $this->id;
    }

    public function getDeviceId(): string
    {
        return $this->deviceId;
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
        $this->signOutReason = null;
    }

    public function getSignOutReason(): ?SignOutReason
    {
        return $this->signOutReason;
    }

    public function isSignedIn(): bool
    {
        return null !== $this->tokenHash && !$this->signOutReason instanceof SignOutReason && !$this->isRemoved();
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

    /**
     * Removes the device (D81). The token hash is kept so an undo can give the
     * install its session back, and so only the real install can be told.
     */
    public function remove(\DateTimeImmutable $at): void
    {
        $this->removedAt ??= $at;
    }

    public function noticeRemoval(\DateTimeImmutable $at): void
    {
        $this->removalNoticedAt ??= $at;
    }

    public function canRestore(): bool
    {
        return $this->isRemoved() && !$this->removalNoticedAt instanceof \DateTimeImmutable;
    }

    public function restore(): void
    {
        $this->removedAt = null;
    }
}
