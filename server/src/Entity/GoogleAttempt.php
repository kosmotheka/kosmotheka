<?php

declare(strict_types=1);

namespace App\Entity;

use App\Repository\GoogleAttemptRepository;
use Doctrine\ORM\Mapping as ORM;

/**
 * One try at «Continuar con Google» (D77). The app opens it with the hash of a secret that never leaves the app; the
 * browser goes through Google; the server hands a one-use code back, and the app exchanges it with the secret.
 *
 * Phases: `created` (the app started it), `connected` (the browser was sent to Google; `stateHash` and `binderHash`
 * say which browser), `done` (Google answered; the code, if any, waits for the app). Nothing here is Google's: no
 * token, no `sub`; only our own secrets, and only as hashes except the PKCE one, which dies with the attempt.
 */
#[ORM\Entity(repositoryClass: GoogleAttemptRepository::class)]
#[ORM\Table(name: 'google_attempt')]
#[ORM\Index(name: 'idx_google_attempt_expires', columns: ['expires_at'])]
#[ORM\UniqueConstraint(name: 'uniq_google_attempt_state', columns: ['state_hash'])]
class GoogleAttempt
{
    public const string CREATED = 'created';
    public const string CONNECTED = 'connected';
    public const string DONE = 'done';

    public const int MAX_FAILURES = 5;

    #[ORM\Column(length: 16)]
    private string $phase = self::CREATED;

    /** SHA-256 of the `state` sent to Google, hex. */
    #[ORM\Column(length: 64, nullable: true)]
    private ?string $stateHash = null;

    /** SHA-256 of the cookie that binds the state to the browser that started, hex. */
    #[ORM\Column(length: 64, nullable: true)]
    private ?string $binderHash = null;

    /** Between the server and Google only (PKCE); gone when the phase is `done`. */
    #[ORM\Column(length: 200, nullable: true)]
    private ?string $pkceVerifier = null;

    /** SHA-256 of the one-use code the app is given, hex. */
    #[ORM\Column(length: 64, nullable: true)]
    private ?string $codeHash = null;

    #[ORM\Column(nullable: true)]
    private ?\DateTimeImmutable $codeExpiresAt = null;

    #[ORM\ManyToOne(targetEntity: User::class)]
    #[ORM\JoinColumn(name: 'user_id', nullable: true, onDelete: 'CASCADE', foreignKeyName: 'fk_google_attempt_user')]
    private ?User $user = null;

    /** Wrong secrets tried against the code; at five the attempt is given up. */
    #[ORM\Column]
    private int $failures = 0;

    public function __construct(
        #[ORM\Id]
        #[ORM\Column(length: 64)]
        private readonly string $id,
        /** SHA-256 of the app's secret (`verifier`), base64url. */
        #[ORM\Column(length: 64)]
        private readonly string $verifierHash,
        #[ORM\Column(length: 5)]
        private readonly string $locale,
        #[ORM\Column]
        private readonly \DateTimeImmutable $expiresAt,
    ) {
    }

    public function getId(): string
    {
        return $this->id;
    }

    public function getVerifierHash(): string
    {
        return $this->verifierHash;
    }

    public function getLocale(): string
    {
        return $this->locale;
    }

    public function getExpiresAt(): \DateTimeImmutable
    {
        return $this->expiresAt;
    }

    public function getPhase(): string
    {
        return $this->phase;
    }

    public function getStateHash(): ?string
    {
        return $this->stateHash;
    }

    public function getBinderHash(): ?string
    {
        return $this->binderHash;
    }

    public function getPkceVerifier(): ?string
    {
        return $this->pkceVerifier;
    }

    public function getCodeHash(): ?string
    {
        return $this->codeHash;
    }

    public function getCodeExpiresAt(): ?\DateTimeImmutable
    {
        return $this->codeExpiresAt;
    }

    public function getUser(): ?User
    {
        return $this->user;
    }

    public function getFailures(): int
    {
        return $this->failures;
    }

    public function connect(string $stateHash, string $binderHash, string $pkceVerifier): void
    {
        $this->phase = self::CONNECTED;
        $this->stateHash = $stateHash;
        $this->binderHash = $binderHash;
        $this->pkceVerifier = $pkceVerifier;
    }

    /** Google answered: the PKCE secret is spent, and the attempt can only be finished (or fail) from here. */
    public function close(): void
    {
        $this->phase = self::DONE;
        $this->pkceVerifier = null;
    }

    public function grant(User $user, string $codeHash, \DateTimeImmutable $codeExpiresAt): void
    {
        $this->user = $user;
        $this->codeHash = $codeHash;
        $this->codeExpiresAt = $codeExpiresAt;
    }

    public function noteFailure(): void
    {
        ++$this->failures;
    }
}
