<?php

declare(strict_types=1);

namespace App\Controller;

use App\Entity\User;
use App\Security\CurrentDevice;
use App\Security\RateLimitKey;
use App\Sync\EventsToken;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpKernel\Attribute\RateLimit;
use Symfony\Component\Routing\Attribute\Route;
use Symfony\Component\Security\Http\Attribute\CurrentUser;

/**
 * `GET /v1/cloud/events` (milestone 7.3, D75): the token a device subscribes to the live
 * notices with. It reads nothing from the request: the topics are the account's own.
 */
final class CloudEventsController extends AbstractController
{
    public function __construct(
        private readonly EventsToken $eventsToken,
        private readonly CurrentDevice $currentDevice,
    ) {
    }

    #[Route('/v1/cloud/events', methods: ['GET'])]
    #[RateLimit('cloud_events', key: RateLimitKey::account(...))]
    public function token(#[CurrentUser] User $user): JsonResponse
    {
        return new JsonResponse($this->eventsToken->issue($user, $this->currentDevice->get()));
    }
}
