<?php

declare(strict_types=1);

namespace App\Entity;

use App\Repository\MembershipRepository;
use Doctrine\ORM\Mapping as ORM;
use Symfony\Component\Uid\Uuid;

/**
 * The access table (D84, D76): an account may reach a cloud library only if a
 * row says so. Until milestone 7.5 the only row is the owner's, created with
 * the library. The role is always the member's, never copied here.
 */
#[ORM\Entity(repositoryClass: MembershipRepository::class)]
#[ORM\Table(name: 'membership')]
#[ORM\UniqueConstraint(name: 'uniq_membership_user_cloud_library', columns: ['user_id', 'cloud_library_id'])]
#[ORM\UniqueConstraint(name: 'uniq_membership_cloud_library_member', columns: ['cloud_library_id', 'member_id'])]
#[ORM\Index(name: 'idx_membership_user', columns: ['user_id'])]
#[ORM\Index(name: 'idx_membership_cloud_library', columns: ['cloud_library_id'])]
final class Membership
{
    #[ORM\Id]
    #[ORM\Column(type: 'uuid', unique: true)]
    private Uuid $id;

    #[ORM\Column]
    private \DateTimeImmutable $createdAt;

    public function __construct(
        #[ORM\ManyToOne(targetEntity: User::class)]
        #[ORM\JoinColumn(name: 'user_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_membership_user')]
        private readonly User $user,
        #[ORM\ManyToOne(targetEntity: CloudLibrary::class)]
        #[ORM\JoinColumn(name: 'cloud_library_id', nullable: false, onDelete: 'CASCADE', foreignKeyName: 'fk_membership_cloud_library')]
        private readonly CloudLibrary $cloudLibrary,
        /** The library's `member` row this account is (D76); set once, by AccountLinker (7.5.2b-2); one account per person in a library. */
        #[ORM\Column(type: 'uuid', nullable: true)]
        private ?Uuid $memberId = null,
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

    public function getCloudLibrary(): CloudLibrary
    {
        return $this->cloudLibrary;
    }

    public function getMemberId(): ?Uuid
    {
        return $this->memberId;
    }

    /**
     * Says which person this account is, once (the owner's row is made before the library has persons).
     *
     * @throws \LogicException when the row already names another person
     */
    public function linkTo(Uuid $memberId): void
    {
        if ($this->memberId instanceof Uuid && !$this->memberId->equals($memberId)) {
            throw new \LogicException('This membership already names another person.');
        }
        $this->memberId = $memberId;
    }

    public function getCreatedAt(): \DateTimeImmutable
    {
        return $this->createdAt;
    }
}
