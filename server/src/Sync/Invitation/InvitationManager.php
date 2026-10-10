<?php

declare(strict_types=1);

namespace App\Sync\Invitation;

use App\Dto\CreateInvitationRequest;
use App\Entity\Catalog\Library;
use App\Entity\CloudLibrary;
use App\Entity\Invitation;
use App\Entity\User;
use App\Enum\InvitationKind;
use App\Repository\InvitationRepository;
use App\Security\AccountException;
use App\Security\AccountMailer;
use App\Security\RequestLimiter;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;
use Symfony\Component\Uid\Uuid;

/**
 * Makes, lists and revokes the invitations of a library (D43). Only whoever administers it: that is read from the
 * server's tables (`InvitationRules`), never from anything the client says. The server creates no persons: the one
 * an invitation is for was created before, as a reader, by an operation the administrator's device signed.
 */
final readonly class InvitationManager
{
    public const string LIFETIME = '+24 hours';

    public function __construct(
        private EntityManagerInterface $entityManager,
        private InvitationRepository $invitations,
        private InvitationRules $rules,
        private EmailFingerprint $fingerprints,
        private AccountMailer $mailer,
        private InvitationSettings $settings,
        private RequestLimiter $limiter,
        private ClockInterface $clock,
    ) {
    }

    /**
     * @return array{invitation: Invitation, link: ?string} the link only for the `link` kind: an `email` one travels
     *                                                      by mail alone, so not even who made it holds the link
     *
     * @throws AccountException
     */
    public function create(CloudLibrary $library, User $by, CreateInvitationRequest $request): array
    {
        if (!$this->settings->isEnabled()) {
            throw AccountException::invitationsUnavailable();
        }
        $this->limiter->consumeInvitationCreate($by->getId()->toRfc4122());
        $this->rules->requireAdministrator($library, $by);
        $this->rules->requireReady($library);
        $member = Uuid::fromString($request->memberId);
        $this->rules->requireInviteable($library, $member, $request->kind);
        $email = InvitationKind::Email === $request->kind ? $request->email : null;
        if ((InvitationKind::Email === $request->kind) !== (null !== $email)) {
            throw AccountException::invalidRequest();
        }
        $fingerprint = null === $email ? null : $this->fingerprints->of($email);
        if (null !== $fingerprint) {
            $this->limiter->consumeInvitationEmail($by->getId()->toRfc4122(), $fingerprint);
        }

        $token = InvitationToken::generate();
        $invitation = new Invitation(
            $library,
            $member,
            $request->kind,
            InvitationToken::fingerprint($token),
            $by,
            $this->now()->modify(self::LIFETIME),
            $fingerprint,
        );
        $link = $this->settings->linkFor($token);
        // The mail goes inside the transaction: if it cannot be sent, the invitation is not left behind.
        $this->entityManager->wrapInTransaction(function () use ($invitation, $email, $by, $library, $link): void {
            $this->entityManager->persist($invitation);
            $this->entityManager->flush();
            if (null !== $email) {
                $this->mailer->sendInvitation($email, $by->getLocale(), $this->libraryName($library), $link);
            }
        });

        return ['invitation' => $invitation, 'link' => null === $email ? $link : null];
    }

    /**
     * @return list<array{id: string, kind: string, memberId: string, state: string, createdAt: string, expiresAt: string, acceptedAt: ?string}>
     *
     * @throws AccountException not an administrator
     */
    public function list(CloudLibrary $library, User $by): array
    {
        $this->rules->requireAdministrator($library, $by);
        $now = $this->now();

        return array_map(fn (Invitation $invitation): array => [
            'id' => $invitation->getId()->toRfc4122(),
            'kind' => $invitation->getKind()->value,
            'memberId' => $invitation->getMemberId()->toRfc4122(),
            'state' => $this->state($invitation, $now),
            'createdAt' => $invitation->getCreatedAt()->format(\DateTimeInterface::ATOM),
            'expiresAt' => $invitation->getExpiresAt()->format(\DateTimeInterface::ATOM),
            'acceptedAt' => $invitation->getAcceptedAt()?->format(\DateTimeInterface::ATOM),
        ], $this->invitations->allOf($library));
    }

    /**
     * Revokes an invitation of THIS library: the id is compared with its invitations, not looked up alone (D84).
     * Revoking one that is already revoked or expired is not an error; one that was accepted cannot be taken back.
     *
     * @throws AccountException not an administrator, not found, or already used
     */
    public function revoke(CloudLibrary $library, User $by, string $id): void
    {
        $this->rules->requireAdministrator($library, $by);
        $invitation = Uuid::isValid($id) ? $this->invitations->findOneIn($library, Uuid::fromString($id)) : null;
        if (!$invitation instanceof Invitation) {
            throw AccountException::notFound();
        }
        if ($invitation->isAccepted()) {
            throw AccountException::invitationUsed();
        }
        $invitation->revoke($this->now());
        $this->entityManager->flush();
    }

    public function libraryName(CloudLibrary $library): string
    {
        $row = $this->entityManager->createQuery(\sprintf('SELECT l.name AS name FROM %s l WHERE l.cloudLibrary = :library', Library::class))
            ->setParameter('library', $library->getId(), 'uuid')->setMaxResults(1)->getOneOrNullResult();

        return \is_array($row) && \is_string($row['name']) ? $row['name'] : '';
    }

    private function state(Invitation $invitation, \DateTimeImmutable $now): string
    {
        return match (true) {
            $invitation->isAccepted() => 'accepted',
            $invitation->isRevoked() => 'revoked',
            $invitation->isExpired($now) => 'expired',
            default => 'pending',
        };
    }

    private function now(): \DateTimeImmutable
    {
        return $this->clock->now()->setTimezone(new \DateTimeZone('UTC'));
    }
}
