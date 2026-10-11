<?php

declare(strict_types=1);

namespace App\EventListener;

use Symfony\Component\EventDispatcher\Attribute\AsEventListener;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Event\ExceptionEvent;
use Symfony\Component\HttpKernel\KernelEvents;
use Symfony\Component\Serializer\Exception\ExtraAttributesException;

/**
 * A body with a field the route does not know is the client's mistake: `422`, not the `500` the Serializer's exception
 * would otherwise become (`#[MapRequestPayload]` with strict fields lets it out).
 */
#[AsEventListener(event: KernelEvents::EXCEPTION)]
final class InvalidBodyListener
{
    public function __invoke(ExceptionEvent $event): void
    {
        $exception = $event->getThrowable();
        if (!$exception instanceof ExtraAttributesException) {
            return;
        }

        $event->setResponse(new JsonResponse(['error' => 'invalid_request'], Response::HTTP_UNPROCESSABLE_ENTITY));
    }
}
