<?php

declare(strict_types=1);

namespace App\Controller;

use App\Dto\CreateInvitationRequest;
use App\Dto\OperationPayload;
use App\Entity\User;
use App\Security\CurrentDevice;
use App\Security\RequestLimiter;
use App\Sync\CloudLibraryAccess;
use App\Sync\Invitation\InvitationAcceptor;
use App\Sync\Invitation\InvitationLookup;
use App\Sync\Invitation\InvitationManager;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\Routing\Attribute\Route;
use Symfony\Component\Security\Http\Attribute\IsGranted;

/**
 * Invitations to a cloud library (milestone 7.6.1, D43, docs/architecture/cloud.md). Thin: the rules live in
 * App\Sync\Invitation, and every library route goes through the door of the cloud libraries first. The only route
 * without a session is looking at an invitation by its token (`security.yaml`), which is why `IsGranted` is on each
 * method and not on the class.
 */
#[Route('/v1/cloud')]
final class InvitationController extends AbstractController
{
    public function __construct(
        private readonly CloudLibraryAccess $access,
        private readonly CurrentDevice $currentDevice,
        private readonly InvitationManager $manager,
        private readonly InvitationLookup $lookup,
        private readonly InvitationAcceptor $acceptor,
        private readonly RequestLimiter $limiter,
    ) {
    }

    #[IsGranted('ROLE_USER')]
    #[Route('/libraries/{id}/invitations', methods: ['POST'])]
    public function create(string $id, #[MapRequestPayload(serializationContext: OperationPayload::SERIALIZATION_CONTEXT)] CreateInvitationRequest $payload): JsonResponse
    {
        $user = $this->currentUser();
        $library = $this->access->require($id, $user, $this->currentDevice->get());
        $created = $this->manager->create($library, $user, $payload);
        $invitation = $created['invitation'];

        return new JsonResponse([
            'id' => $invitation->getId()->toRfc4122(),
            'kind' => $invitation->getKind()->value,
            'memberId' => $invitation->getMemberId()->toRfc4122(),
            'expiresAt' => $invitation->getExpiresAt()->format(\DateTimeInterface::ATOM),
            'link' => $created['link'],
        ], Response::HTTP_CREATED);
    }

    #[IsGranted('ROLE_USER')]
    #[Route('/libraries/{id}/invitations', methods: ['GET'])]
    public function list(string $id): JsonResponse
    {
        $user = $this->currentUser();
        $library = $this->access->require($id, $user, $this->currentDevice->get());

        return new JsonResponse(['invitations' => $this->manager->list($library, $user)]);
    }

    #[IsGranted('ROLE_USER')]
    #[Route('/libraries/{id}/invitations/{invitationId}', methods: ['DELETE'])]
    public function revoke(string $id, string $invitationId): JsonResponse
    {
        $user = $this->currentUser();
        $library = $this->access->require($id, $user, $this->currentDevice->get());
        $this->manager->revoke($library, $user, $invitationId);

        return new JsonResponse(null, Response::HTTP_NO_CONTENT);
    }

    /** Without a session: only what a link may tell whoever holds it. Limited per address. */
    #[Route('/invitations/{token}', methods: ['GET'])]
    public function show(string $token, Request $request): JsonResponse
    {
        $this->limiter->consumeInvitationLookup((string) $request->getClientIp());

        return new JsonResponse($this->lookup->peek($token));
    }

    #[IsGranted('ROLE_USER')]
    #[Route('/invitations/{token}/accept', methods: ['POST'])]
    public function accept(string $token, Request $request): JsonResponse
    {
        $user = $this->currentUser();
        $this->limiter->consumeInvitationLookup((string) $request->getClientIp());
        $this->limiter->consumeInvitationAccept($user->getId()->toRfc4122());
        $invitation = $this->acceptor->accept($token, $user);

        return new JsonResponse(['id' => $invitation->getCloudLibrary()->getId()->toRfc4122()]);
    }

    private function currentUser(): User
    {
        $user = $this->getUser();
        \assert($user instanceof User);

        return $user;
    }
}
