<?php

declare(strict_types=1);

namespace App\Controller;

use App\Entity\User;
use App\Security\CurrentDevice;
use App\Security\RequestLimiter;
use App\Sync\EventsToken;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\Routing\Attribute\Route;
use Symfony\Component\Security\Http\Attribute\IsGranted;

/**
 * `GET /v1/cloud/events` (milestone 7.3, D75): the token a device subscribes to the live
 * notices with. It reads nothing from the request: the topics are the account's own.
 */
#[IsGranted('ROLE_USER')]
final class CloudEventsController extends AbstractController
{
    public function __construct(
        private readonly EventsToken $eventsToken,
        private readonly CurrentDevice $currentDevice,
        private readonly RequestLimiter $limiter,
    ) {
    }

    #[Route('/v1/cloud/events', methods: ['GET'])]
    public function token(): JsonResponse
    {
        $user = $this->getUser();
        \assert($user instanceof User);
        $this->limiter->consumeCloudEvents($user->getId()->toRfc4122());

        return new JsonResponse($this->eventsToken->issue($user, $this->currentDevice->get()));
    }
}
