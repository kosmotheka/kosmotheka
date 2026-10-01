<?php

declare(strict_types=1);

namespace App\EventListener;

use App\Security\AccountException;
use Symfony\Component\EventDispatcher\Attribute\AsEventListener;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpKernel\Event\ExceptionEvent;
use Symfony\Component\HttpKernel\KernelEvents;

/** Turns an AccountException into the account API's `{"error": "<code>"}` body. */
#[AsEventListener(event: KernelEvents::EXCEPTION)]
final class AccountExceptionListener
{
    public function __invoke(ExceptionEvent $event): void
    {
        $exception = $event->getThrowable();
        if (!$exception instanceof AccountException) {
            return;
        }

        $response = new JsonResponse(['error' => $exception->errorCode->value], $exception->statusCode);
        if (null !== $exception->retryAfterSeconds) {
            $response->headers->set('Retry-After', (string) $exception->retryAfterSeconds);
        }

        $event->setResponse($response);
    }
}
