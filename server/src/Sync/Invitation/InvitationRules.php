<?php

declare(strict_types=1);

namespace App\Sync\Invitation;

use App\Entity\Catalog\Copy;
use App\Entity\Catalog\Device as CatalogDevice;
use App\Entity\Catalog\Mark;
use App\Entity\Catalog\Member;
use App\Entity\CloudLibrary;
use App\Entity\Invitation;
use App\Entity\Membership;
use App\Entity\User;
use App\Enum\InvitationKind;
use App\Enum\Role;
use App\Security\AccountException;
use App\Sync\CloudAuthor;
use App\Sync\Permission\CatalogLibraryView;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Uid\Uuid;

/**
 * Who may invite, to which library and for which person (D43, amended on 10/10/2026). One place, asked twice: when an
 * invitation is made and, again, when it is looked at or accepted, because by then the world may have moved (the
 * one who invited was demoted or thrown out, the person got a device or an account, the roles were turned off).
 *
 * Everything is read from the server's own tables and from the catalog of THAT cloud library, as scalars (the merge
 * engine writes below the ORM, so an entity in memory may be stale). A person's id comes from the client and is only
 * compared with that catalog (D84).
 */
final readonly class InvitationRules
{
    public function __construct(
        private EntityManagerInterface $entityManager,
        private CloudAuthor $author,
    ) {
    }

    /**
     * Whether the account administers the library now: its access row names a person who has the role `admin` in the
     * catalog and still has access. The role is the person's, read here, never copied (D76).
     */
    public function administers(CloudLibrary $library, User $user): bool
    {
        if ($user->isBlocked()) {
            return false;
        }
        $row = $this->entityManager->createQuery(\sprintf('SELECT ms.memberId AS member, ms.withdrawnAt AS withdrawn FROM %s ms WHERE ms.cloudLibrary = :library AND ms.user = :user', Membership::class))
            ->setParameter('library', $library->getId(), 'uuid')->setParameter('user', $user)->setMaxResults(1)->getOneOrNullResult();
        if (!\is_array($row) || null !== $row['withdrawn'] || !$row['member'] instanceof Uuid) {
            return false;
        }
        $person = $this->person($library, $row['member']);

        return null !== $person && Role::Admin->value === $person['role'] && true !== $person['withdrawn'];
    }

    public function requireAdministrator(CloudLibrary $library, User $user): void
    {
        if (!$this->administers($library, $user)) {
            throw AccountException::invitationForbidden();
        }
    }

    /**
     * Whether the library can be invited to: linked, with persons and roles (without them anybody does anything, and
     * so would whoever comes in by a link), and with the cloud able to link devices (the power of D76), so the
     * person keeps their role from the first device.
     */
    public function isReady(CloudLibrary $library): bool
    {
        return $library->isLinked()
            && new CatalogLibraryView($this->entityManager, $library)->hasRoles()
            && $this->author->power($library) instanceof Uuid;
    }

    public function requireReady(CloudLibrary $library): void
    {
        if (!$this->isReady($library)) {
            throw AccountException::invitationLibraryNotReady();
        }
    }

    /**
     * Whether this person of THIS library may be invited in this way. Never an administrator (a stolen email would
     * be enough to throw the others out, D98), nor someone with the access withdrawn, nor someone who already has an
     * account, in the catalog or in the access table. A person already in the library is invited by email and keeps
     * their role; a link is for a new reader only, because it gives its person to whoever opens it first.
     */
    public function isInviteable(CloudLibrary $library, Uuid $member, InvitationKind $kind): bool
    {
        $person = $this->person($library, $member);
        if (null === $person || true === $person['withdrawn'] || (\is_string($person['account']) && '' !== $person['account'])) {
            return false;
        }
        $allowed = InvitationKind::Link === $kind ? [Role::Reader->value] : [Role::Editor->value, Role::Reader->value];
        if (!\in_array($person['role'], $allowed, true) || $this->hasAnAccount($library, $member)) {
            return false;
        }

        return InvitationKind::Email === $kind || $this->isNew($library, $member);
    }

    public function requireInviteable(CloudLibrary $library, Uuid $member, InvitationKind $kind): void
    {
        if (!$this->isInviteable($library, $member, $kind)) {
            throw AccountException::invitationPersonUnsuitable();
        }
    }

    /**
     * What an invitation is worth now. A used, revoked or expired one is said as such; one whose conditions stopped
     * holding (the one who made it no longer administers, the library lost its roles or the cloud its power, the
     * person changed) is treated as revoked: the app asks for another one.
     */
    public function requireUsable(Invitation $invitation, \DateTimeImmutable $now): void
    {
        if ($invitation->isAccepted()) {
            throw AccountException::invitationUsed();
        }
        if ($invitation->isRevoked()) {
            throw AccountException::invitationRevoked();
        }
        if ($invitation->isExpired($now)) {
            throw AccountException::invitationExpired();
        }
        $library = $invitation->getCloudLibrary();
        if (!$this->administers($library, $invitation->getCreatedBy())
            || !$this->isReady($library)
            || !$this->isInviteable($library, $invitation->getMemberId(), $invitation->getKind())) {
            throw AccountException::invitationRevoked();
        }
    }

    /**
     * A person who has nothing of their own in the library: no device assigned, no copy, no mark. «New», for a link:
     * a leaked link must not let a stranger take the place of someone who already has books and history there.
     */
    private function isNew(CloudLibrary $library, Uuid $member): bool
    {
        $own = [
            \sprintf('SELECT 1 FROM %s x JOIN x.member p WHERE x.cloudLibrary = :library AND p.cloudLibrary = :library AND p.id = :member', CatalogDevice::class),
            \sprintf('SELECT 1 FROM %s x JOIN x.owner p WHERE x.cloudLibrary = :library AND p.cloudLibrary = :library AND p.id = :member', Copy::class),
            \sprintf('SELECT 1 FROM %s x JOIN x.member p WHERE x.cloudLibrary = :library AND p.cloudLibrary = :library AND p.id = :member', Mark::class),
        ];

        return array_all($own, fn (string $dql): bool => [] === $this->entityManager->createQuery($dql)->setParameter('library', $library->getId(), 'uuid')->setParameter('member', $member->toRfc4122())->setMaxResults(1)->getResult());
    }

    private function hasAnAccount(CloudLibrary $library, Uuid $member): bool
    {
        return [] !== $this->entityManager->createQuery(\sprintf('SELECT 1 FROM %s ms WHERE ms.cloudLibrary = :library AND ms.memberId = :member AND ms.withdrawnAt IS NULL', Membership::class))
            ->setParameter('library', $library->getId(), 'uuid')->setParameter('member', $member, 'uuid')->setMaxResults(1)->getResult();
    }

    /**
     * @return ?array{role: ?string, withdrawn: ?bool, account: ?string} null if the library's catalog has no such person
     */
    private function person(CloudLibrary $library, Uuid $member): ?array
    {
        $row = $this->entityManager->createQuery(\sprintf('SELECT m.role AS role, m.accessWithdrawn AS withdrawn, m.accountId AS account FROM %s m WHERE m.cloudLibrary = :library AND m.id = :member', Member::class))
            ->setParameter('library', $library->getId(), 'uuid')->setParameter('member', $member->toRfc4122())->setMaxResults(1)->getOneOrNullResult();
        if (!\is_array($row)) {
            return null;
        }
        \assert(null === $row['role'] || \is_string($row['role']));
        \assert(null === $row['withdrawn'] || \is_bool($row['withdrawn']));
        \assert(null === $row['account'] || \is_string($row['account']));

        return ['role' => $row['role'], 'withdrawn' => $row['withdrawn'], 'account' => $row['account']];
    }
}
