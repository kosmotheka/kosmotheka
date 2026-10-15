<?php

declare(strict_types=1);

namespace App\Controller;

use App\Dto\GoogleFinishRequest;
use App\Dto\GoogleStartRequest;
use App\Entity\User;
use App\Enum\ActivityKind;
use App\Enum\DeviceProofPurpose;
use App\Security\AccountException;
use App\Security\ActivityLog;
use App\Security\DeviceProof;
use App\Security\DeviceSignIn;
use App\Security\Google\GoogleAttempts;
use App\Security\LoginChallengeIssuer;
use App\Security\RequestLimiter;
use App\Security\UserChecker;
use App\Sync\Invitation\EmailFingerprint;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\Cookie;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\RedirectResponse;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\Routing\Attribute\Route;
use Symfony\Component\Routing\Generator\UrlGeneratorInterface;

/**
 * «Continuar con Google» (D77, docs/architecture/accounts.md). `start` and `finish` are the app's; `connect` and
 * `callback` are the browser's, and carry secrets in their address (the attempt, Google's code): they must stay out of
 * the access log (docs/development/server.md). Thin: the rules are in `GoogleAttempts` and `GoogleAccounts`.
 */
#[Route('/v1/auth/google')]
final class GoogleController extends AbstractController
{
    private const string COOKIE = 'kosmotheka_google';

    public function __construct(
        private readonly GoogleAttempts $attempts,
        private readonly RequestLimiter $requestLimiter,
        private readonly LoginChallengeIssuer $challengeIssuer,
        private readonly DeviceProof $deviceProof,
        private readonly DeviceSignIn $deviceSignIn,
        private readonly UserChecker $userChecker,
        private readonly EntityManagerInterface $entityManager,
        private readonly ActivityLog $activityLog,
        private readonly EmailFingerprint $fingerprints,
    ) {
    }

    #[Route('/start', name: 'google_start', methods: ['POST'])]
    public function start(#[MapRequestPayload] GoogleStartRequest $payload, Request $request): JsonResponse
    {
        $this->requestLimiter->consumeGoogle((string) $request->getClientIp());

        $started = $this->attempts->start($payload->verifierHash, $payload->locale);

        return new JsonResponse([
            'attempt' => $started['attempt'],
            'url' => $this->generateUrl('google_connect', ['attempt' => $started['attempt']], UrlGeneratorInterface::ABSOLUTE_URL),
            'expires_at' => $started['expiresAt']->format(\DateTimeInterface::ATOM),
        ]);
    }

    #[Route('/connect', name: 'google_connect', methods: ['GET'])]
    public function connect(Request $request): Response
    {
        $this->requestLimiter->consumeGoogle((string) $request->getClientIp());
        if (!$this->attempts->isConfigured()) {
            return $this->page('unavailable', Response::HTTP_NOT_FOUND);
        }

        $connected = $this->attempts->connect((string) $request->query->get('attempt', ''));
        if (null === $connected) {
            return $this->page('expired', Response::HTTP_BAD_REQUEST);
        }

        $response = new RedirectResponse($connected['url']);
        $response->headers->setCookie(Cookie::create(self::COOKIE, $connected['binder'], new \DateTimeImmutable('+10 minutes'), '/v1/auth/google', null, $request->isSecure(), true, false, Cookie::SAMESITE_LAX));
        $this->noReferrer($response);

        return $response;
    }

    #[Route('/callback', name: 'google_callback', methods: ['GET'])]
    public function callback(Request $request): Response
    {
        $this->requestLimiter->consumeGoogle((string) $request->getClientIp());
        if (!$this->attempts->isConfigured()) {
            return $this->page('unavailable', Response::HTTP_NOT_FOUND);
        }

        $error = $request->query->get('error');
        $code = $request->query->get('code');
        $back = $this->attempts->receive(
            (string) $request->query->get('state', ''),
            (string) $request->cookies->get(self::COOKIE, ''),
            \is_string($code) ? $code : null,
            \is_string($error) ? $error : null,
        );
        if (null === $back) {
            // Not a state this server gave to this browser: nobody to send back to, and nothing is said of why.
            return $this->page('expired', Response::HTTP_BAD_REQUEST);
        }

        $response = $this->page('back', Response::HTTP_OK, 'kosmotheka://auth/google?'.http_build_query($back));
        $response->headers->clearCookie(self::COOKIE, '/v1/auth/google');

        return $response;
    }

    #[Route('/finish', name: 'google_finish', methods: ['POST'])]
    public function finish(#[MapRequestPayload] GoogleFinishRequest $payload, Request $request): JsonResponse
    {
        $this->requestLimiter->consumeGoogle((string) $request->getClientIp());
        if (!$this->attempts->isConfigured()) {
            throw AccountException::googleUnavailable();
        }

        // The same proof a login makes: a challenge that is spent whatever happens, signed by the device's own key.
        $this->challengeIssuer->consume($payload->challenge);
        $this->deviceProof->verify(DeviceProofPurpose::Google, $payload->challenge, $payload->deviceId, $payload->publicKey, $payload->signature);

        $attempt = $this->attempts->claim($payload->attempt, $payload->code, $payload->verifier);
        $user = $attempt->getUser();
        \assert($user instanceof User);

        $this->userChecker->checkPreAuth($user);
        $this->requestLimiter->requireLoginOpen($this->fingerprints->of($user->getEmail()));
        // Google does not replace the second factor: whoever turned it on did it so that one key would not be enough.
        $this->deviceSignIn->requireTwoFactor($user, $payload->totpCode);

        $device = $this->deviceSignIn->device($user, $payload->deviceId, $payload->deviceName, $payload->publicKey);
        $token = $this->deviceSignIn->issueToken($device);
        $this->attempts->done($attempt);
        $this->activityLog->record(ActivityKind::LoginSucceededWithGoogle, $user, $device->getDeviceId());
        $this->entityManager->flush();

        return new JsonResponse([
            'token' => $token,
            'user' => $this->deviceSignIn->userPayload($user),
        ]);
    }

    /** The page the browser is left on. It says nothing of why a thing failed, and it is not kept nor passed on. */
    private function page(string $kind, int $status, ?string $appLink = null): Response
    {
        $text = match ($kind) {
            'back' => ['You can go back to Kosmotheka.', 'Ya puedes volver a Kosmotheka.'],
            'unavailable' => ['Signing in with Google is not available on this server.', 'Entrar con Google no está disponible en este servidor.'],
            default => ['This link no longer works. Start again from the app.', 'Este enlace ya no vale. Empieza de nuevo desde la app.'],
        };
        $link = null === $appLink ? '' : \sprintf(
            '<meta http-equiv="refresh" content="0;url=%1$s"><p><a href="%1$s">Kosmotheka</a></p>',
            htmlspecialchars($appLink, \ENT_QUOTES),
        );
        $html = \sprintf(
            '<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="robots" content="noindex"><title>Kosmotheka</title>%s</head><body style="font-family:sans-serif;margin:2rem;max-width:32rem"><p>%s</p><p>%s</p></body></html>',
            $link,
            htmlspecialchars($text[0], \ENT_QUOTES),
            htmlspecialchars($text[1], \ENT_QUOTES),
        );
        $response = new Response($html, $status, ['Content-Type' => 'text/html; charset=utf-8']);
        $this->noReferrer($response);

        return $response;
    }

    /** What is in the address (the attempt, the code) must not travel on to any other site, nor be kept. */
    private function noReferrer(Response $response): void
    {
        $response->headers->set('Referrer-Policy', 'no-referrer');
        $response->headers->set('Cache-Control', 'no-store');
        $response->headers->set('X-Robots-Tag', 'noindex');
    }
}
