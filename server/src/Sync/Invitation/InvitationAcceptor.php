<?php

declare(strict_types=1);

namespace App\Sync\Invitation;

use App\Entity\Invitation;
use App\Entity\Membership;
use App\Entity\User;
use App\Enum\InvitationKind;
use App\Repository\InvitationRepository;
use App\Repository\MembershipRepository;
use App\Security\AccountException;
use App\Sync\AccountLinker;
use App\Sync\SyncNotifier;
use Doctrine\DBAL\LockMode;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;
use Symfony\Component\Uid\Uuid;

/**
 * Accepts an invitation: the account becomes the invited person of that library (D43). It is the second way of
 * linking an account with a person, next to a device with a vouched key (D96): here, whoever vouches is the
 * administrator who invited. The device of the account is linked afterwards, by the usual `claim`.
 *
 * Everything is checked again inside the lock of the library, the same one pushes and claims take, and the
 * invitation row is read again under its own lock: two accounts that open the same link at once take turns, and the
 * second reads «used». Nothing the client sends but the token decides anything (D84).
 */
final readonly class InvitationAcceptor
{
    public function __construct(
        private EntityManagerInterface $entityManager,
        private InvitationRepository $invitations,
        private MembershipRepository $memberships,
        private InvitationRules $rules,
        private EmailFingerprint $fingerprints,
        private AccountLinker $linker,
        private SyncNotifier $notifier,
        private ClockInterface $clock,
    ) {
    }

    /**
     * @throws AccountException
     */
    public function accept(string $token, User $user): Invitation
    {
        if (!InvitationToken::isWellFormed($token)) {
            throw AccountException::notFound();
        }
        $found = $this->invitations->findOneByTokenHash(InvitationToken::fingerprint($token)) ?? throw AccountException::notFound();

        $written = false;
        $invitation = $this->entityManager->wrapInTransaction(function () use ($found, $user, &$written): Invitation {
            // The lock of every push and claim, then the row itself; what follows is read inside them.
            $library = $found->getCloudLibrary();
            $this->entityManager->refresh($library, LockMode::PESSIMISTIC_WRITE);
            $this->entityManager->refresh($found, LockMode::PESSIMISTIC_WRITE);
            $this->entityManager->refresh($found->getCreatedBy());
            $now = $this->clock->now()->setTimezone(new \DateTimeZone('UTC'));
            $this->rules->requireUsable($found, $now);
            $this->requireItsEmail($found, $user);

            $member = $found->getMemberId();
            $membership = $this->memberships->findOf($library, $user);
            if ($membership instanceof Membership) {
                $this->entityManager->refresh($membership);
            }
            // An account that is already a person here stays that person: an invitation does not move it.
            if ($membership instanceof Membership && !$membership->isWithdrawn() && $membership->getMemberId() instanceof Uuid) {
                throw AccountException::invitationAccountAlreadyInLibrary();
            }
            if ($membership instanceof Membership) {
                $membership->linkTo($member);
            } else {
                $this->entityManager->persist(new Membership($user, $library, $member));
            }
            $found->accept($user, $now);
            $this->entityManager->flush();
            // The person's `account_id`, written by the cloud if it has the power (it was checked), like `claim` does.
            $written = $this->linker->noteAccount($library, $user, $member);

            return $found;
        });

        $library = $invitation->getCloudLibrary();
        if ($written) {
            $this->notifier->libraryChanged($library);
        }
        $this->notifier->invitationAccepted($invitation->getCreatedBy(), $library);
        $this->notifier->librariesOfAccountChanged($user);

        return $invitation;
    }

    /** An invitation tied to an email is for the account whose verified email it is; compared in constant time. */
    private function requireItsEmail(Invitation $invitation, User $user): void
    {
        if (InvitationKind::Email !== $invitation->getKind()) {
            return;
        }
        $expected = $invitation->getEmailHash();
        if (null === $expected || !$user->isConfirmed() || !hash_equals($expected, $this->fingerprints->of($user->getEmail()))) {
            throw AccountException::invitationNotForThisAccount();
        }
    }
}
