<?php

declare(strict_types=1);

namespace App\Sync\Invitation;

use App\Repository\InvitationRepository;
use App\Security\AccountException;
use Symfony\Component\Clock\ClockInterface;

/**
 * What anybody holding a link may learn from it, without a session: the name of the library and the kind of
 * invitation, or why it is worth nothing. Not the persons, not the roles, not who invites (their name or email would
 * leak with a link that lands in a chat). Whoever does not hold a real token only gets `not_found`.
 */
final readonly class InvitationLookup
{
    public function __construct(
        private InvitationRepository $invitations,
        private InvitationRules $rules,
        private InvitationManager $manager,
        private ClockInterface $clock,
    ) {
    }

    /**
     * @return array{libraryName: string, kind: string, expiresAt: string}
     *
     * @throws AccountException not found; or why the invitation is worth nothing
     */
    public function peek(string $token): array
    {
        if (!InvitationToken::isWellFormed($token)) {
            throw AccountException::notFound();
        }
        $invitation = $this->invitations->findOneByTokenHash(InvitationToken::fingerprint($token)) ?? throw AccountException::notFound();
        $this->rules->requireUsable($invitation, $this->clock->now()->setTimezone(new \DateTimeZone('UTC')));

        return [
            'libraryName' => $this->manager->libraryName($invitation->getCloudLibrary()),
            'kind' => $invitation->getKind()->value,
            'expiresAt' => $invitation->getExpiresAt()->format(\DateTimeInterface::ATOM),
        ];
    }
}
