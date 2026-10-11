<?php

declare(strict_types=1);

namespace App\Entity;

use App\Enum\ActivityKind;
use App\Repository\ActivityEntryRepository;
use Doctrine\ORM\Mapping as ORM;
use Symfony\Component\Uid\Uuid;

/**
 * One line of the activity log (D66, 7.7): what happened, who did it, when and from where. **Append-only**: no
 * setter, and no route, repository method or command changes or deletes a row, except the purge by age
 * (`ActivityEntryRepository::deleteBefore`).
 *
 * **No foreign keys** on purpose: every id is copied here as a value, so the line outlives the account, the library
 * or the device it is about. Written only by `App\Security\ActivityLog`, which takes the author, the time and the
 * place from the server, never from what a client says (D84).
 */
#[ORM\Entity(repositoryClass: ActivityEntryRepository::class)]
#[ORM\Table(name: 'activity_log')]
#[ORM\Index(name: 'idx_activity_log_occurred_at', columns: ['occurred_at'])]
#[ORM\Index(name: 'idx_activity_log_account', columns: ['account_id'])]
#[ORM\Index(name: 'idx_activity_log_library', columns: ['library_id'])]
final class ActivityEntry
{
    #[ORM\Id]
    #[ORM\Column(type: 'uuid', unique: true)]
    private Uuid $id;

    public function __construct(
        #[ORM\Column(length: 32, enumType: ActivityKind::class)]
        private ActivityKind $kind,
        /** The server's clock, UTC: never the operation's or the client's. */
        #[ORM\Column]
        private \DateTimeImmutable $occurredAt,
        /** `http` for a request, `console` for a command: a command has no IP. */
        #[ORM\Column(length: 8)]
        private string $channel,
        /** The account (`user.id`) that did it, or, for what the cloud does by itself, the one it was done for. */
        #[ORM\Column(type: 'uuid', nullable: true)]
        private ?Uuid $accountId = null,
        /** Keyed hash of the account's email, so «was it this address?» can be answered after the account is gone; never the address. */
        #[ORM\Column(length: 64, nullable: true)]
        private ?string $accountEmailFingerprint = null,
        #[ORM\Column(length: 64, nullable: true)]
        private ?string $deviceId = null,
        /** The cloud library (`cloud_library.id`) it happened in. */
        #[ORM\Column(type: 'uuid', nullable: true)]
        private ?Uuid $libraryId = null,
        /** What the server had as the library's name at that moment, so the line is understood after it is deleted. */
        #[ORM\Column(length: 200, nullable: true)]
        private ?string $libraryName = null,
        /** The person, device or invitation it affected, if any. */
        #[ORM\Column(length: 64, nullable: true)]
        private ?string $subjectId = null,
        /** The operation that said it, for what comes from an operation. */
        #[ORM\Column(type: 'uuid', nullable: true)]
        private ?Uuid $operationId = null,
        /** A short closed value that completes the kind (a role, `true`, the kind of invitation, a person's id). Never free text. */
        #[ORM\Column(length: 64, nullable: true)]
        private ?string $detail = null,
        #[ORM\Column(length: 45, nullable: true)]
        private ?string $ip = null,
        #[ORM\Column(length: 255, nullable: true)]
        private ?string $userAgent = null,
    ) {
        $this->id = Uuid::v7();
    }

    public function getId(): Uuid
    {
        return $this->id;
    }

    public function getKind(): ActivityKind
    {
        return $this->kind;
    }

    public function getOccurredAt(): \DateTimeImmutable
    {
        return $this->occurredAt;
    }

    public function getChannel(): string
    {
        return $this->channel;
    }

    public function getAccountId(): ?Uuid
    {
        return $this->accountId;
    }

    public function getAccountEmailFingerprint(): ?string
    {
        return $this->accountEmailFingerprint;
    }

    public function getDeviceId(): ?string
    {
        return $this->deviceId;
    }

    public function getLibraryId(): ?Uuid
    {
        return $this->libraryId;
    }

    public function getLibraryName(): ?string
    {
        return $this->libraryName;
    }

    public function getSubjectId(): ?string
    {
        return $this->subjectId;
    }

    public function getOperationId(): ?Uuid
    {
        return $this->operationId;
    }

    public function getDetail(): ?string
    {
        return $this->detail;
    }

    public function getIp(): ?string
    {
        return $this->ip;
    }

    public function getUserAgent(): ?string
    {
        return $this->userAgent;
    }
}
