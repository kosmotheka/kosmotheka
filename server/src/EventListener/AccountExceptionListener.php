<?php

declare(strict_types=1);

namespace App\EventListener;

use App\Enum\AccountErrorCode;
use App\Security\AccountException;
use Symfony\Component\EventDispatcher\Attribute\AsEventListener;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpKernel\Event\ExceptionEvent;
use Symfony\Component\HttpKernel\Exception\TooManyRequestsHttpException;
use Symfony\Component\HttpKernel\KernelEvents;

/**
 * Turns an AccountException into the account API's `{"error": "<code>"}` body, and the 429 of a `#[RateLimit]`
 * attribute (a TooManyRequestsHttpException) into the same body the limits taken by hand give.
 */
#[AsEventListener(event: KernelEvents::EXCEPTION)]
final class AccountExceptionListener
{
    public function __invoke(ExceptionEvent $event): void
    {
        $exception = $event->getThrowable();
        if ($exception instanceof TooManyRequestsHttpException) {
            $response = new JsonResponse(['error' => AccountErrorCode::TooManyRequests->value], $exception->getStatusCode(), $exception->getHeaders());
            $event->setResponse($response);

            return;
        }
        if (!$exception instanceof AccountException) {
            return;
        }

        $response = new JsonResponse(['error' => $exception->errorCode->value] + $exception->extra, $exception->statusCode);
        if (null !== $exception->retryAfterSeconds) {
            $response->headers->set('Retry-After', (string) $exception->retryAfterSeconds);
        }

        $event->setResponse($response);
    }
}
