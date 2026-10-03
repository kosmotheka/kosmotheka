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
use App\Security\TwoFactor;
use App\Security\UserChecker;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
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
        private readonly DeviceRepository $deviceRepository,
        private readonly UserPasswordHasherInterface $passwordHasher,
        private readonly PasswordHasherFactoryInterface $hasherFactory,
        private readonly UserChecker $userChecker,
        private readonly EntityManagerInterface $entityManager,
        private readonly RequestLimiter $requestLimiter,
        private readonly CurrentDevice $currentDevice,
        private readonly TwoFactor $twoFactor,
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
        $this->checkTwoFactor($user, $payload->totpCode);

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
     * Hashes the password with the users' own hasher and throws the result away, so
     * an unknown email costs one hash, as long as checking a wrong password.
     */
    private function hashDummyPassword(string $password): void
    {
        $this->hasherFactory->getPasswordHasher(User::class)->hash($password);
    }

    /**
     * D78: asked after credentials and account status, before the device
     * itself. The challenge is already consumed by this point, so a wrong or
     * missing code means the app has to request a new one before retrying.
     *
     * @throws AccountException with two_factor_required or invalid_two_factor_code
     */
    private function checkTwoFactor(User $user, ?string $totpCode): void
    {
        if (!$user->isTwoFactorEnabled()) {
            return;
        }

        if (null === $totpCode) {
            throw AccountException::twoFactorRequired();
        }

        $this->twoFactor->verify($user, $totpCode);
    }

    /** @throws AccountException with device_removed, device_taken or device_key_mismatch */
    private function signInDevice(User $user, string $deviceId, string $deviceName, string $publicKey): Device
    {
        // A device is its id together with its key: the id alone is public
        // (D84, SEG-016), and the key is what the login just proved.
        $device = $this->deviceRepository->findOneByDeviceIdAndKey($deviceId, $publicKey);
        if (!$device instanceof Device) {
            if ($this->deviceRepository->findOneOfUser($user, $deviceId) instanceof Device) {
                throw AccountException::deviceKeyMismatch();
            }

            // Another account's row with this id and another key is somebody
            // else's install (or an intruder's useless one): it does not matter.
            $device = new Device($deviceId, $user, $deviceName, $publicKey);
            $this->entityManager->persist($device);

            return $device;
        }

        // Before device_taken: a removed install must start again (D81),
        // whichever account tries it.
        if ($device->isRemoved()) {
            $device->noticeRemoval(new \DateTimeImmutable('now', new \DateTimeZone('UTC')));
            $this->entityManager->flush();

            throw AccountException::deviceRemoved();
        }

        if (!$device->getUser()->getId()->equals($user->getId())) {
            throw AccountException::deviceTaken();
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
