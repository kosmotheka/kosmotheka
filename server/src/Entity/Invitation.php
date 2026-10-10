<?php

declare(strict_types=1);

namespace App\Entity;

use App\Enum\InvitationKind;
use App\Repository\InvitationRepository;
use Doctrine\ORM\Mapping as ORM;
use Symfony\Component\Uid\Uuid;

/**
 * An invitation to a cloud library (D43). It hangs from the cloud library (D84), names a person of that library and
 * keeps only the fingerprint of its token, and of the email when it is tied to one. «One use» is a rule of the code
 * (`InvitationAcceptor`), not of the table, so a later kind of invitation with several uses needs no new schema.
 */
#[ORM\Entity(repositoryClass: InvitationRepository::class)]
#[ORM\Table(name: 'invitation')]
#[ORM\UniqueConstraint(name: 'uniq_invitation_token_hash', columns: ['token_hash'])]
#[ORM\Index(name: 'idx_invitation_cloud_library', columns: ['cloud_library_id'])]
#[ORM\Index(name: 'idx_invitation_created_by', columns: ['created_by_id'])]
#[ORM\Index(name: 'idx_invitation_accepted_by', columns: ['accepted_by_id'])]
final class Invitation
{
    // Not `readonly`: the acceptor reads the row again under its lock (`EntityManager::refresh`), which writes the properties.

    #[ORM\Id]
    #[ORM\Column(type: 'uuid', unique: true)]
    private Uuid $id;

    #[ORM\Column]
    private \DateTimeImmutable $createdAt;

    #[ORM\Column(nullable: true)]
    private ?\DateTimeImmutable $acceptedAt = null;

    #[ORM\ManyToOne(targetEntity: User::class)]
    #[ORM\JoinColumn(name: 'accepted_by_id', nullable: true, onDelete: 'SET NULL', foreignKeyName: 'fk_invitation_accepted_by')]
    private ?User $acceptedBy = null;

    #[ORM\Column(nullable: true)]
    private ?\DateTimeImmutable $revokedAt = null;

    public function __construct(
        #[ORM\ManyToOne(targetEntity: CloudLibrary::class)]
        #[ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_invitation_cloud_library')]
        private CloudLibrary $cloudLibrary,
        /** The person of that library (`member.id`) the invitation is for; the client's id, compared with the library's own catalog (D84). */
        #[ORM\Column(type: 'uuid')]
        private Uuid $memberId,
        #[ORM\Column(length: 8, enumType: InvitationKind::class)]
        private InvitationKind $kind,
        /** SHA-256 of the token, in hexadecimal. The token is random, so it needs no salt. */
        #[ORM\Column(length: 64)]
        private string $tokenHash,
        /** The administrator who made it: if they stop administering, the invitation stops being worth anything. */
        #[ORM\ManyToOne(targetEntity: User::class)]
        #[ORM\JoinColumn(name: 'created_by_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_invitation_created_by')]
        private User $createdBy,
        #[ORM\Column]
        private \DateTimeImmutable $expiresAt,
        /** HMAC of the normalised email, only for the `email` kind (an email is guessed, so it is keyed with a secret of the server). */
        #[ORM\Column(length: 64, nullable: true)]
        private ?string $emailHash = null,
    ) {
        $this->id = Uuid::v7();
        $this->createdAt = new \DateTimeImmutable('now', new \DateTimeZone('UTC'));
    }

    public function getId(): Uuid
    {
        return $this->id;
    }

    public function getCloudLibrary(): CloudLibrary
    {
        return $this->cloudLibrary;
    }

    public function getMemberId(): Uuid
    {
        return $this->memberId;
    }

    public function getKind(): InvitationKind
    {
        return $this->kind;
    }

    public function getTokenHash(): string
    {
        return $this->tokenHash;
    }

    public function getEmailHash(): ?string
    {
        return $this->emailHash;
    }

    public function getCreatedBy(): User
    {
        return $this->createdBy;
    }

    public function getCreatedAt(): \DateTimeImmutable
    {
        return $this->createdAt;
    }

    public function getExpiresAt(): \DateTimeImmutable
    {
        return $this->expiresAt;
    }

    public function getAcceptedAt(): ?\DateTimeImmutable
    {
        return $this->acceptedAt;
    }

    public function getAcceptedBy(): ?User
    {
        return $this->acceptedBy;
    }

    public function getRevokedAt(): ?\DateTimeImmutable
    {
        return $this->revokedAt;
    }

    public function isAccepted(): bool
    {
        return $this->acceptedAt instanceof \DateTimeImmutable;
    }

    public function isRevoked(): bool
    {
        return $this->revokedAt instanceof \DateTimeImmutable;
    }

    public function isExpired(\DateTimeImmutable $now): bool
    {
        return $this->expiresAt <= $now;
    }

    public function accept(User $by, \DateTimeImmutable $at): void
    {
        $this->acceptedAt = $at;
        $this->acceptedBy = $by;
    }

    public function revoke(\DateTimeImmutable $at): void
    {
        $this->revokedAt ??= $at;
    }
}
