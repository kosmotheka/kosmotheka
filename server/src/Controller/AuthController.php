<?php

declare(strict_types=1);

namespace App\Controller;

use App\Dto\LoginRequest;
use App\Entity\User;
use App\Enum\ActivityKind;
use App\Enum\DeviceProofPurpose;
use App\Repository\UserRepository;
use App\Security\AccountException;
use App\Security\ActivityLog;
use App\Security\CurrentDevice;
use App\Security\DeviceProof;
use App\Security\DeviceSignIn;
use App\Security\LoginChallengeIssuer;
use App\Security\RateLimitKey;
use App\Security\RequestLimiter;
use App\Security\UserChecker;
use App\Sync\Invitation\EmailFingerprint;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\HttpKernel\Attribute\RateLimit;
use Symfony\Component\PasswordHasher\Hasher\PasswordHasherFactoryInterface;
use Symfony\Component\PasswordHasher\Hasher\UserPasswordHasherInterface;
use Symfony\Component\Routing\Attribute\Route;

/**
 * Device sign-in (milestone 7.1, part 2, docs/architecture/accounts.md):
 * the login challenge, login itself, and signing a device out. Thin: maps,
 * calls services, returns JSON.
 */
#[Route('/v1/auth')]
final class AuthController extends AbstractController
{
    public function __construct(
        private readonly LoginChallengeIssuer $challengeIssuer,
        private readonly DeviceProof $deviceProof,
        private readonly UserRepository $userRepository,
        private readonly DeviceSignIn $deviceSignIn,
        private readonly UserPasswordHasherInterface $passwordHasher,
        private readonly PasswordHasherFactoryInterface $hasherFactory,
        private readonly UserChecker $userChecker,
        private readonly EntityManagerInterface $entityManager,
        private readonly RequestLimiter $requestLimiter,
        private readonly CurrentDevice $currentDevice,
        private readonly ActivityLog $activityLog,
        private readonly EmailFingerprint $fingerprints,
    ) {
    }

    #[Route('/challenge', methods: ['POST'])]
    #[RateLimit('login', key: RateLimitKey::ip(...))]
    public function challenge(): JsonResponse
    {
        $issued = $this->challengeIssuer->issue();

        return new JsonResponse([
            'challenge' => $issued['challenge'],
            'expires_at' => $issued['expiresAt']->format(\DateTimeInterface::ATOM),
        ]);
    }

    #[Route('/login', methods: ['POST'])]
    #[RateLimit('login', key: RateLimitKey::ip(...))]
    public function login(#[MapRequestPayload] LoginRequest $payload): JsonResponse
    {
        // Order matters (docs/architecture/accounts.md): challenge, then proof,
        // then credentials, then account status, then the device itself.
        $this->challengeIssuer->consume($payload->challenge);
        $this->deviceProof->verify(DeviceProofPurpose::Login, $payload->challenge, $payload->deviceId, $payload->publicKey, $payload->signature);

        // Before the password is looked at: a cut account answers the same with the right one, and the same for an address
        // with no account (SEG-051).
        $email = $this->normaliseEmail($payload->email);
        $this->requestLimiter->requireLoginOpen($this->fingerprints->of($email));
        $user = $this->userRepository->findOneByEmail($email);
        $this->checkCredentials($user, $email, $payload->password);
        \assert($user instanceof User);

        $this->userChecker->checkPreAuth($user);
        $this->deviceSignIn->requireTwoFactor($user, $payload->totpCode);

        $device = $this->deviceSignIn->device($user, $payload->deviceId, $payload->deviceName, $payload->publicKey);
        $token = $this->deviceSignIn->issueToken($device);
        $this->activityLog->record(ActivityKind::LoginSucceeded, $user, $device->getDeviceId());
        $this->entityManager->flush();

        return new JsonResponse([
            'token' => $token,
            'user' => $this->deviceSignIn->userPayload($user),
        ]);
    }

    #[Route('/logout', methods: ['POST'])]
    public function logout(): Response
    {
        $device = $this->currentDevice->get();
        $device->setTokenHash(null);
        $this->activityLog->record(ActivityKind::DeviceSignedOut, $device->getUser(), $device->getDeviceId());
        $this->entityManager->flush();

        return new Response(status: Response::HTTP_NO_CONTENT);
    }

    /** @throws AccountException with invalid_credentials, never revealing which check failed */
    private function checkCredentials(?User $user, string $email, string $password): void
    {
        if (!$user instanceof User || null === $user->getPassword()) {
            $this->hashDummyPassword($password);
            $this->requestLimiter->noteLoginFailure($this->fingerprints->of($email));
            // Written too, so that failing against an account that exists costs the same as against one that does not.
            $this->activityLog->recordNow(ActivityKind::LoginFailed, $user);

            throw AccountException::invalidCredentials();
        }

        if (!$this->passwordHasher->isPasswordValid($user, $password)) {
            $this->requestLimiter->noteLoginFailure($this->fingerprints->of($email));
            // An account that exists, and somebody who does not know its password. The device is not named: it was only
            // proved to hold its own key, and nothing says it is the account's (D84).
            $this->activityLog->recordNow(ActivityKind::LoginFailed, $user);

            throw AccountException::invalidCredentials();
        }

        if ($this->passwordHasher->needsRehash($user)) {
            $this->userRepository->upgradePassword($user, $this->passwordHasher->hashPassword($user, $password));
        }
    }

    /**
     * Hashes the password with the users' own hasher and throws the result away, so
     * an unknown email costs one hash, as long as checking a wrong password.
     */
    private function hashDummyPassword(string $password): void
    {
        $this->hasherFactory->getPasswordHasher(User::class)->hash($password);
    }

    private function normaliseEmail(string $email): string
    {
        return mb_strtolower(trim($email));
    }
}
