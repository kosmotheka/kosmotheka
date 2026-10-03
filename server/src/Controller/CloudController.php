<?php

declare(strict_types=1);

namespace App\Controller;

use App\Dto\PullRequest;
use App\Dto\PushRequest;
use App\Entity\CloudLibrary;
use App\Entity\User;
use App\Repository\CloudLibraryRepository;
use App\Sync\CloudLibraryAccess;
use App\Sync\CloudLibraryCreator;
use App\Sync\CloudPull;
use App\Sync\CloudPush;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\Routing\Attribute\Route;
use Symfony\Component\Security\Http\Attribute\IsGranted;

/**
 * The cloud libraries of the signed-in account and their sync (milestone 7.2,
 * docs/architecture/cloud.md). Thin: every `{id}` route asks CloudLibraryAccess
 * first, and the rules live in App\Sync.
 */
#[Route('/v1/cloud/libraries')]
#[IsGranted('ROLE_USER')]
final class CloudController extends AbstractController
{
    public function __construct(
        private readonly CloudLibraryAccess $access,
        private readonly CloudLibraryCreator $creator,
        private readonly CloudLibraryRepository $libraries,
        private readonly CloudPull $cloudPull,
        private readonly CloudPush $cloudPush,
    ) {
    }

    #[Route('', methods: ['POST'])]
    public function create(): JsonResponse
    {
        $library = $this->creator->create($this->currentUser());

        return new JsonResponse(['id' => $library->getId()->toRfc4122()], Response::HTTP_CREATED);
    }

    #[Route('', methods: ['GET'])]
    public function list(): JsonResponse
    {
        return new JsonResponse(array_map(
            static fn (CloudLibrary $library): array => [
                'id' => $library->getId()->toRfc4122(),
                'localLibraryId' => $library->getLocalLibraryId(),
            ],
            $this->libraries->findAllAccessibleBy($this->currentUser()),
        ));
    }

    #[Route('/{id}/pull', methods: ['POST'])]
    public function pull(string $id, #[MapRequestPayload] PullRequest $payload): JsonResponse
    {
        $library = $this->access->require($id, $this->currentUser());

        return new JsonResponse($this->cloudPull->pull($library, $payload));
    }

    #[Route('/{id}/push', methods: ['POST'])]
    public function push(string $id, Request $request): JsonResponse
    {
        $library = $this->access->require($id, $this->currentUser());

        return new JsonResponse($this->cloudPush->push($library, PushRequest::fromJson($request->getContent()))->toArray());
    }

    private function currentUser(): User
    {
        $user = $this->getUser();
        \assert($user instanceof User);

        return $user;
    }
}
