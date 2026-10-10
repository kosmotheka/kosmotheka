<?php

declare(strict_types=1);

namespace App\Entity;

use App\Repository\UserRepository;
use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;
use Symfony\Component\Security\Core\User\PasswordAuthenticatedUserInterface;
use Symfony\Component\Security\Core\User\UserInterface;
use Symfony\Component\Uid\Uuid;

/** An account on the server (persona->user in docs/architecture/accounts.md). */
#[ORM\Entity(repositoryClass: UserRepository::class)]
#[ORM\Table(name: '`user`')]
#[ORM\UniqueConstraint(name: 'uniq_user_email', columns: ['email'])]
#[ORM\UniqueConstraint(name: 'uniq_user_account_id', columns: ['account_id'])]
final class User implements UserInterface, PasswordAuthenticatedUserInterface
{
    #[ORM\Id]
    #[ORM\Column(type: 'uuid', unique: true)]
    private Uuid $id;

    /**
     * The only thing of the account that travels with a library (`member.account_id`, D76): random, so it says
     * nothing of the email or of the primary key. Never changes.
     */
    #[ORM\Column(type: 'uuid')]
    private Uuid $accountId;

    /** Null for a Google-only account (later milestone). */
    #[ORM\Column(nullable: true)]
    private ?string $password = null;

    /** @var list<string> */
    #[ORM\Column(type: Types::JSON)]
    private array $roles = [];

    #[ORM\Column]
    private \DateTimeImmutable $createdAt;

    #[ORM\Column(nullable: true)]
    private ?\DateTimeImmutable $confirmedAt = null;

    #[ORM\Column(nullable: true)]
    private ?\DateTimeImmutable $blockedAt = null;

    /** Base32 secret, set once TOTP is enabled (D78). */
    #[ORM\Column(nullable: true)]
    private ?string $totpSecret = null;

    /** Base32 secret awaiting confirmation (`POST /v1/account/two-factor/setup`), until `enable()` moves it above. */
    #[ORM\Column(nullable: true)]
    private ?string $totpPendingSecret = null;

    #[ORM\Column(nullable: true)]
    private ?\DateTimeImmutable $totpEnabledAt = null;

    /** Anti-replay: a code for a timestep at or before this one is rejected. */
    #[ORM\Column(nullable: true)]
    private ?int $totpLastTimestep = null;

    public function __construct(
        #[ORM\Column(length: 180)]
        private string $email,
        #[ORM\Column(length: 100)]
        private string $name,
        /** `en` or `es`; sent by the app at registration (D49). */
        #[ORM\Column(length: 5)]
        private string $locale,
    ) {
        $this->id = Uuid::v7();
        $this->accountId = Uuid::v4();
        $this->createdAt = new \DateTimeImmutable('now', new \DateTimeZone('UTC'));
    }

    public function getId(): Uuid
    {
        return $this->id;
    }

    public function getAccountId(): Uuid
    {
        return $this->accountId;
    }

    public function getEmail(): string
    {
        return $this->email;
    }

    public function setEmail(string $email): void
    {
        $this->email = $email;
    }

    public function getPassword(): ?string
    {
        return $this->password;
    }

    public function setPassword(string $password): void
    {
        $this->password = $password;
    }

    public function getName(): string
    {
        return $this->name;
    }

    public function setName(string $name): void
    {
        $this->name = $name;
    }

    public function getLocale(): string
    {
        return $this->locale;
    }

    public function setLocale(string $locale): void
    {
        $this->locale = $locale;
    }

    /** @return list<string> */
    public function getRoles(): array
    {
        $roles = $this->roles;
        $roles[] = 'ROLE_USER';

        return array_values(array_unique($roles));
    }

    /** @param list<string> $roles */
    public function setRoles(array $roles): void
    {
        $this->roles = $roles;
    }

    public function getCreatedAt(): \DateTimeImmutable
    {
        return $this->createdAt;
    }

    public function getConfirmedAt(): ?\DateTimeImmutable
    {
        return $this->confirmedAt;
    }

    public function isConfirmed(): bool
    {
        return $this->confirmedAt instanceof \DateTimeImmutable;
    }

    public function confirm(\DateTimeImmutable $at): void
    {
        $this->confirmedAt = $at;
    }

    public function getBlockedAt(): ?\DateTimeImmutable
    {
        return $this->blockedAt;
    }

    public function isBlocked(): bool
    {
        return $this->blockedAt instanceof \DateTimeImmutable;
    }

    public function block(\DateTimeImmutable $at): void
    {
        $this->blockedAt = $at;
    }

    public function unblock(): void
    {
        $this->blockedAt = null;
    }

    /** @return non-empty-string */
    public function getUserIdentifier(): string
    {
        if ('' === $this->email) {
            throw new \LogicException('A user always has an email.');
        }

        return $this->email;
    }

    public function getTotpSecret(): ?string
    {
        return $this->totpSecret;
    }

    public function setTotpSecret(?string $totpSecret): void
    {
        $this->totpSecret = $totpSecret;
    }

    public function getTotpPendingSecret(): ?string
    {
        return $this->totpPendingSecret;
    }

    public function setTotpPendingSecret(?string $totpPendingSecret): void
    {
        $this->totpPendingSecret = $totpPendingSecret;
    }

    public function getTotpEnabledAt(): ?\DateTimeImmutable
    {
        return $this->totpEnabledAt;
    }

    public function setTotpEnabledAt(?\DateTimeImmutable $totpEnabledAt): void
    {
        $this->totpEnabledAt = $totpEnabledAt;
    }

    public function isTwoFactorEnabled(): bool
    {
        return null !== $this->totpSecret;
    }

    public function getTotpLastTimestep(): ?int
    {
        return $this->totpLastTimestep;
    }

    public function setTotpLastTimestep(?int $totpLastTimestep): void
    {
        $this->totpLastTimestep = $totpLastTimestep;
    }
}
