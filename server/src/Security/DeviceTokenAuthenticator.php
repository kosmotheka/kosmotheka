<?php

declare(strict_types=1);

namespace App\Security;

use App\Entity\Device;
use App\Entity\User;
use App\Enum\AccountErrorCode;
use App\Enum\SignOutReason;
use App\Repository\DeviceRepository;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\Security\Core\Authentication\Token\TokenInterface;
use Symfony\Component\Security\Core\Exception\AuthenticationException;
use Symfony\Component\Security\Core\Exception\CustomUserMessageAuthenticationException;
use Symfony\Component\Security\Http\Authenticator\AbstractAuthenticator;
use Symfony\Component\Security\Http\Authenticator\Passport\Badge\UserBadge;
use Symfony\Component\Security\Http\Authenticator\Passport\SelfValidatingPassport;
use Symfony\Component\Security\Http\EntryPoint\AuthenticationEntryPointInterface;

/**
 * Authenticates `/v1` calls with `X-Device-Id` and `Authorization: Bearer
 * <token>` (milestone 7.1, part 2, docs/architecture/accounts.md). Doubles as
 * the firewall's entry point, for a request with no credentials at all.
 *
 * The authenticated Device, not only its User, is stored as a request
 * attribute (App\Security\CurrentDevice reads it) because `/v1/account/
 * devices` and logout act on the calling device itself.
 *
 * `/v1/account/password/reset` (part 3) is public but still reads the
 * calling device, when there is one, to leave it signed in. A bad or missing
 * token there must not block the request the way it does everywhere else, so
 * authentication failures on that route are swallowed: returning null from
 * onAuthenticationFailure() leaves the request unauthenticated instead of
 * short-circuiting it with a 401, and PUBLIC_ACCESS lets it through.
 */
final class DeviceTokenAuthenticator extends AbstractAuthenticator implements AuthenticationEntryPointInterface
{
    private const string BEARER_PREFIX = 'Bearer ';
    private const int LAST_SEEN_THROTTLE_SECONDS = 3600;

    /** @var list<string> */
    private const array OPTIONAL_AUTH_PATHS = ['/v1/account/password/reset'];

    public function __construct(
        private readonly DeviceRepository $deviceRepository,
        private readonly EntityManagerInterface $entityManager,
        private readonly ClockInterface $clock,
    ) {
    }

    public function supports(Request $request): bool
    {
        return str_starts_with((string) $request->headers->get('Authorization'), self::BEARER_PREFIX);
    }

    public function authenticate(Request $request): SelfValidatingPassport
    {
        $deviceId = $request->headers->get('X-Device-Id');
        $token = substr((string) $request->headers->get('Authorization'), \strlen(self::BEARER_PREFIX));

        if (null === $deviceId || '' === $token) {
            throw new CustomUserMessageAuthenticationException('Missing device id or token.');
        }

        // Several rows may share a device id, so the token finds the row and
        // the header must then agree with it.
        $device = $this->deviceRepository->findOneByTokenHash(hash('sha256', $token));
        if (!$device instanceof Device || !hash_equals($device->getDeviceId(), $deviceId)) {
            throw new CustomUserMessageAuthenticationException('Unknown device, signed-out device or wrong token.');
        }

        // Only after the token matched: device ids are visible to everyone
        // sharing a library, and telling a stranger would end the undo (D81).
        if ($device->isRemoved()) {
            $device->noticeRemoval($this->clock->now()->setTimezone(new \DateTimeZone('UTC')));
            $this->entityManager->flush();

            throw new DeviceRemovedException();
        }

        $reason = $device->getSignOutReason();
        if ($reason instanceof SignOutReason) {
            throw new SignedOutException($reason);
        }

        $request->attributes->set(CurrentDevice::ATTRIBUTE, $device);
        $this->touchLastSeen($device);

        $user = $device->getUser();

        return new SelfValidatingPassport(new UserBadge($user->getUserIdentifier(), static fn (): User => $user));
    }

    public function onAuthenticationSuccess(Request $request, TokenInterface $token, string $firewallName): ?Response
    {
        return null;
    }

    public function onAuthenticationFailure(Request $request, AuthenticationException $exception): ?JsonResponse
    {
        if (\in_array($request->getPathInfo(), self::OPTIONAL_AUTH_PATHS, true)) {
            return null;
        }

        $code = match (true) {
            $exception instanceof DeviceRemovedException => AccountErrorCode::DeviceRemoved,
            $exception instanceof SignedOutException => match ($exception->reason) {
                SignOutReason::PasswordChanged => AccountErrorCode::SignedOutPasswordChanged,
            },
            default => AccountErrorCode::InvalidToken,
        };

        return new JsonResponse(['error' => $code->value], Response::HTTP_UNAUTHORIZED);
    }

    /** No credentials at all on a protected route. */
    public function start(Request $request, ?AuthenticationException $authException = null): JsonResponse
    {
        return new JsonResponse(['error' => AccountErrorCode::AuthenticationRequired->value], Response::HTTP_UNAUTHORIZED);
    }

    private function touchLastSeen(Device $device): void
    {
        $now = $this->clock->now()->setTimezone(new \DateTimeZone('UTC'));
        $lastSeenAt = $device->getLastSeenAt();
        if ($lastSeenAt instanceof \DateTimeImmutable && $now->getTimestamp() - $lastSeenAt->getTimestamp() < self::LAST_SEEN_THROTTLE_SECONDS) {
            return;
        }

        $device->touch($now);
        $this->entityManager->flush();
    }
}
