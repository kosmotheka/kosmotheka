<?php

declare(strict_types=1);

namespace App\Controller;

use App\Dto\CoverInventoryRequest;
use App\Entity\User;
use App\Security\CurrentDevice;
use App\Security\RateLimitKey;
use App\Sync\CloudCovers;
use App\Sync\CloudLibraryAccess;
use App\Sync\CoverRejectedException;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\HttpKernel\Attribute\RateLimit;
use Symfony\Component\Routing\Attribute\Route;
use Symfony\Component\Security\Http\Attribute\CurrentUser;
use Symfony\Component\Serializer\Normalizer\AbstractNormalizer;

/**
 * The cover files of a cloud library (D82, docs/architecture/cloud.md). Thin, like CloudController: each route asks
 * CloudLibraryAccess first, and the rules live in App\Sync\CloudCovers.
 */
#[Route('/v1/cloud/libraries/{id}/covers')]
final class CloudCoverController extends AbstractController
{
    public function __construct(
        private readonly CloudLibraryAccess $access,
        private readonly CloudCovers $covers,
        private readonly CurrentDevice $currentDevice,
    ) {
    }

    #[Route('/inventory', methods: ['POST'])]
    #[RateLimit('cloud_covers', key: RateLimitKey::account(...))]
    public function inventory(string $id, #[MapRequestPayload(serializationContext: [AbstractNormalizer::ALLOW_EXTRA_ATTRIBUTES => false])] CoverInventoryRequest $payload, #[CurrentUser] User $user): JsonResponse
    {
        $library = $this->access->require($id, $user, $this->currentDevice->get());

        return new JsonResponse($this->covers->swap($library, $payload->have));
    }

    #[Route('/{name}', requirements: ['name' => CloudCovers::NAME], methods: ['GET'])]
    #[RateLimit('cloud_covers', key: RateLimitKey::account(...))]
    public function download(string $id, string $name, #[CurrentUser] User $user): Response
    {
        $library = $this->access->require($id, $user, $this->currentDevice->get());
        try {
            $bytes = $this->covers->read($library, $name);
        } catch (CoverRejectedException $rejected) {
            return $this->rejected($rejected);
        }

        return new Response($bytes, Response::HTTP_OK, ['Content-Type' => 'application/octet-stream', 'X-Content-Type-Options' => 'nosniff']);
    }

    #[Route('/{name}', requirements: ['name' => CloudCovers::NAME], methods: ['PUT'])]
    #[RateLimit('cloud_covers', key: RateLimitKey::account(...))]
    public function upload(string $id, string $name, Request $request, #[CurrentUser] User $user): Response
    {
        $library = $this->access->require($id, $user, $this->currentDevice->get());
        try {
            $this->covers->keep($library, $name, $request->getContent());
        } catch (CoverRejectedException $rejected) {
            return $this->rejected($rejected);
        }

        return new Response(null, Response::HTTP_NO_CONTENT);
    }

    private function rejected(CoverRejectedException $rejected): JsonResponse
    {
        return new JsonResponse(['code' => $rejected->reason->value], $rejected->reason->status());
    }
}
