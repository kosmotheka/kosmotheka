<?php

declare(strict_types=1);

namespace App\Entity;

use App\Enum\EmailCodePurpose;
use App\Repository\EmailCodeRepository;
use Doctrine\ORM\Mapping as ORM;
use Symfony\Component\Uid\Uuid;

/**
 * A live 6-digit code sent by email (D49, D50): confirming an address or
 * resetting a password. One per user and purpose; App\Security\EmailCodeIssuer
 * owns issuing, sending and verifying it.
 */
#[ORM\Entity(repositoryClass: EmailCodeRepository::class)]
#[ORM\Table(name: 'email_code')]
#[ORM\UniqueConstraint(name: 'uniq_email_code_user_purpose', columns: ['user_id', 'purpose'])]
#[ORM\Index(name: 'idx_email_code_user', columns: ['user_id'])]
final class EmailCode
{
    #[ORM\Id]
    #[ORM\Column(type: 'uuid', unique: true)]
    private Uuid $id;

    #[ORM\Column]
    private int $attempts = 0;

    #[ORM\Column]
    private \DateTimeImmutable $createdAt;

    public function __construct(
        #[ORM\ManyToOne(targetEntity: User::class)]
        #[ORM\JoinColumn(name: 'user_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_email_code_user')]
        private User $user,
        #[ORM\Column(enumType: EmailCodePurpose::class)]
        private EmailCodePurpose $purpose,
        /** HMAC-SHA256 of the code with kernel.secret, hex; never the code itself. */
        #[ORM\Column(length: 64)]
        private string $codeHash,
        #[ORM\Column]
        private \DateTimeImmutable $expiresAt,
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

    public function getPurpose(): EmailCodePurpose
    {
        return $this->purpose;
    }

    public function getCodeHash(): string
    {
        return $this->codeHash;
    }

    public function getExpiresAt(): \DateTimeImmutable
    {
        return $this->expiresAt;
    }

    public function isExpired(\DateTimeImmutable $now): bool
    {
        return $now > $this->expiresAt;
    }

    public function getAttempts(): int
    {
        return $this->attempts;
    }
}
