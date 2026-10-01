<?php

declare(strict_types=1);

namespace App\Controller;

use App\Dto\LoginRequest;
use App\Entity\Device;
use App\Entity\User;
use App\Repository\DeviceRepository;
use App\Repository\UserRepository;
use App\Security\AccountException;
use App\Security\CurrentDevice;
use App\Security\DeviceProof;
use App\Security\LoginChallengeIssuer;
use App\Security\RequestLimiter;
use App\Security\UserChecker;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
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
    private const string DUMMY_PASSWORD_HASH = 'REMOVED';

    public function __construct(
        private readonly LoginChallengeIssuer $challengeIssuer,
        private readonly DeviceProof $deviceProof,
        private readonly UserRepository $userRepository,
        private readonly DeviceRepository $deviceRepository,
        private readonly UserPasswordHasherInterface $passwordHasher,
        private readonly UserChecker $userChecker,
        private readonly EntityManagerInterface $entityManager,
        private readonly RequestLimiter $requestLimiter,
        private readonly CurrentDevice $currentDevice,
    ) {
    }

    #[Route('/challenge', methods: ['POST'])]
    public function challenge(Request $request): JsonResponse
    {
        $this->requestLimiter->consumeLogin((string) $request->getClientIp());

        $issued = $this->challengeIssuer->issue();

        return new JsonResponse([
            'challenge' => $issued['challenge'],
            'expires_at' => $issued['expiresAt']->format(\DateTimeInterface::ATOM),
        ]);
    }

    #[Route('/login', methods: ['POST'])]
    public function login(#[MapRequestPayload] LoginRequest $payload, Request $request): JsonResponse
    {
        $this->requestLimiter->consumeLogin((string) $request->getClientIp());

        // Order matters (docs/architecture/accounts.md): challenge, then proof,
        // then credentials, then account status, then the device itself.
        $this->challengeIssuer->consume($payload->challenge);
        $this->deviceProof->verify($payload->challenge, $payload->deviceId, $payload->publicKey, $payload->signature);

        $user = $this->userRepository->findOneByEmail($this->normaliseEmail($payload->email));
        $this->checkCredentials($user, $payload->password);
        \assert($user instanceof User);

        $this->userChecker->checkPreAuth($user);

        $device = $this->signInDevice($user, $payload->deviceId, $payload->deviceName, $payload->publicKey);
        $token = $this->issueToken($device);
        $this->entityManager->flush();

        return new JsonResponse([
            'token' => $token,
            'user' => $this->userPayload($user),
        ]);
    }

    #[Route('/logout', methods: ['POST'])]
    public function logout(): Response
    {
        $this->currentDevice->get()->setTokenHash(null);
        $this->entityManager->flush();

        return new Response(status: Response::HTTP_NO_CONTENT);
    }

    /** @throws AccountException with invalid_credentials, never revealing which check failed */
    private function checkCredentials(?User $user, string $password): void
    {
        if (!$user instanceof User || null === $user->getPassword()) {
            $this->hashDummyPassword($password);

            throw AccountException::invalidCredentials();
        }

        if (!$this->passwordHasher->isPasswordValid($user, $password)) {
            throw AccountException::invalidCredentials();
        }

        if ($this->passwordHasher->needsRehash($user)) {
            $this->userRepository->upgradePassword($user, $this->passwordHasher->hashPassword($user, $password));
        }
    }

    /**
     * Verifies against a fixed argon2id hash (PHP's default cost, like the real
     * ones) so an unknown email takes as long as a wrong password.
     */
    private function hashDummyPassword(string $password): void
    {
        password_verify($password, self::DUMMY_PASSWORD_HASH);
    }

    /** @throws AccountException with device_taken or device_key_mismatch */
    private function signInDevice(User $user, string $deviceId, string $deviceName, string $publicKey): Device
    {
        $device = $this->deviceRepository->find($deviceId);
        if (!$device instanceof Device) {
            $device = new Device($deviceId, $user, $deviceName, $publicKey);
            $this->entityManager->persist($device);

            return $device;
        }

        if (!$device->getUser()->getId()->equals($user->getId())) {
            throw AccountException::deviceTaken();
        }

        if (!hash_equals($device->getPublicKey(), $publicKey)) {
            throw AccountException::deviceKeyMismatch();
        }

        $device->setName($deviceName);

        return $device;
    }

    private function issueToken(Device $device): string
    {
        $token = sodium_bin2base64(random_bytes(32), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        $device->setTokenHash(hash('sha256', $token));

        return $token;
    }

    /** @return array{id: string, email: string, name: string, locale: string} */
    private function userPayload(User $user): array
    {
        return [
            'id' => $user->getId()->toRfc4122(),
            'email' => $user->getEmail(),
            'name' => $user->getName(),
            'locale' => $user->getLocale(),
        ];
    }

    private function normaliseEmail(string $email): string
    {
        return mb_strtolower(trim($email));
    }
}
