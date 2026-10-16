<?php

declare(strict_types=1);

namespace App\Entity;

use App\Repository\BackupCodeRepository;
use Doctrine\ORM\Mapping as ORM;
use Symfony\Component\Uid\Uuid;

/**
 * One of the ten single-use recovery codes issued when TOTP is enabled (D78).
 * Stored as an HMAC-SHA256 hash (`kernel.secret`), never in clear; App\Security\
 * TwoFactor owns issuing and consuming them.
 */
#[ORM\Entity(repositoryClass: BackupCodeRepository::class)]
#[ORM\Table(name: 'backup_code')]
#[ORM\UniqueConstraint(name: 'uniq_backup_code_user_hash', columns: ['user_id', 'code_hash'])]
#[ORM\Index(name: 'idx_backup_code_user', columns: ['user_id'])]
final class BackupCode
{
    #[ORM\Id]
    #[ORM\Column(type: 'uuid', unique: true)]
    private Uuid $id;

    #[ORM\Column]
    private \DateTimeImmutable $createdAt;

    #[ORM\Column(nullable: true)]
    private ?\DateTimeImmutable $usedAt = null;

    public function __construct(
        #[ORM\ManyToOne(targetEntity: User::class)]
        #[ORM\JoinColumn(name: 'user_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_backup_code_user')]
        private User $user,
        /** HMAC-SHA256 of the normalised code (no dash, upper case) with kernel.secret, hex. */
        #[ORM\Column(length: 64)]
        private string $codeHash,
    ) {
        $this->id = Uuid::v7();
        $this->createdAt = new \DateTimeImmutable('now', new \DateTimeZone('UTC'));
    }

    public function getId(): Uuid
    {
        return $this->id;
    }

    public function getUser(): User
    {
        return $this->user;
    }

    public function getCodeHash(): string
    {
        return $this->codeHash;
    }

    public function isUsed(): bool
    {
        return $this->usedAt instanceof \DateTimeImmutable;
    }
}
