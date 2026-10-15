<?php

declare(strict_types=1);

namespace App\Controller;

use App\Dto\ChangePasswordRequest;
use App\Dto\ConfirmAccountRequest;
use App\Dto\DeleteAccountRequest;
use App\Dto\ForgotPasswordRequest;
use App\Dto\RegisterAccountRequest;
use App\Dto\RequestEmailCodeRequest;
use App\Dto\ResetPasswordRequest;
use App\Dto\UpdateAccountRequest;
use App\Entity\Device;
use App\Entity\User;
use App\Enum\ActivityKind;
use App\Enum\EmailCodePurpose;
use App\Enum\SignOutReason;
use App\Repository\BackupCodeRepository;
use App\Repository\DeviceRepository;
use App\Repository\UserRepository;
use App\Security\AccountException;
use App\Security\AccountMailer;
use App\Security\ActivityLog;
use App\Security\CurrentDevice;
use App\Security\EmailCodeIssuer;
use App\Security\RegistrationGate;
use App\Security\RequestLimiter;
use App\Sync\Invitation\EmailFingerprint;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\PasswordHasher\Hasher\UserPasswordHasherInterface;
use Symfony\Component\Routing\Attribute\Route;
use Symfony\Component\Security\Http\Attribute\IsGranted;

/**
 * Account creation, email confirmation (D49) and the account resource itself
 * once signed in (milestone 7.1, part 2). Thin: maps, calls services, returns JSON.
 */
#[Route('/v1/account')]
final class AccountController extends AbstractController
{
    public function __construct(
        private readonly UserRepository $userRepository,
        private readonly EntityManagerInterface $entityManager,
        private readonly UserPasswordHasherInterface $passwordHasher,
        private readonly EmailCodeIssuer $emailCodeIssuer,
        private readonly AccountMailer $accountMailer,
        private readonly RequestLimiter $requestLimiter,
        private readonly CurrentDevice $currentDevice,
        private readonly DeviceRepository $deviceRepository,
        private readonly BackupCodeRepository $backupCodeRepository,
        private readonly ActivityLog $activityLog,
        private readonly RegistrationGate $registrationGate,
        private readonly EmailFingerprint $fingerprints,
    ) {
    }

    #[Route('/register', methods: ['POST'])]
    public function register(#[MapRequestPayload] RegisterAccountRequest $payload, Request $request): JsonResponse
    {
        $this->requestLimiter->consumeRegistration((string) $request->getClientIp());

        $email = $this->normaliseEmail($payload->email);
        // Before anything is created or sent: a server that only lets some addresses in leaves nothing behind for the rest.
        if (!$this->registrationGate->allows($email)) {
            throw AccountException::registrationClosed();
        }
        $user = $this->userRepository->findOneByEmail($email);

        if (!$user instanceof User) {
            $user = new User($email, $payload->name, $payload->locale);
            $user->setPassword($this->passwordHasher->hashPassword($user, $payload->password));
            $this->entityManager->persist($user);
            $this->entityManager->flush();
            $this->emailCodeIssuer->issueAndSend($user, EmailCodePurpose::ConfirmEmail);

            return new JsonResponse(new \stdClass(), Response::HTTP_ACCEPTED);
        }

        if (!$user->isConfirmed()) {
            $user->setPassword($this->passwordHasher->hashPassword($user, $payload->password));
            $user->setName($payload->name);
            $user->setLocale($payload->locale);
            $this->entityManager->flush();
            $this->emailCodeIssuer->issueAndSend($user, EmailCodePurpose::ConfirmEmail);

            return new JsonResponse(new \stdClass(), Response::HTTP_ACCEPTED);
        }

        $this->accountMailer->sendExistingAccountNotice($user);

        return new JsonResponse(new \stdClass(), Response::HTTP_ACCEPTED);
    }

    #[Route('/confirm', methods: ['POST'])]
    public function confirm(#[MapRequestPayload] ConfirmAccountRequest $payload, Request $request): Response
    {
        $this->requestLimiter->consumeEmailCode((string) $request->getClientIp());

        $user = $this->userRepository->findOneByEmail($this->normaliseEmail($payload->email));
        if (!$user instanceof User || $user->isConfirmed()) {
            throw AccountException::invalidCode();
        }

        $this->emailCodeIssuer->verify($user, EmailCodePurpose::ConfirmEmail, $payload->code);

        $user->confirm(new \DateTimeImmutable('now', new \DateTimeZone('UTC')));
        $this->activityLog->record(ActivityKind::AccountRegistered, $user);
        $this->entityManager->flush();

        return new Response(status: Response::HTTP_NO_CONTENT);
    }

    #[Route('/code', methods: ['POST'])]
    public function requestCode(#[MapRequestPayload] RequestEmailCodeRequest $payload, Request $request): JsonResponse
    {
        $this->requestLimiter->consumeEmailCode((string) $request->getClientIp());

        $user = $this->userRepository->findOneByEmail($this->normaliseEmail($payload->email));
        if ($user instanceof User && !$user->isConfirmed()) {
            $this->emailCodeIssuer->issueAndSend($user, $payload->getPurpose());
        }

        return new JsonResponse(new \stdClass(), Response::HTTP_ACCEPTED);
    }

    #[Route('/password/forgot', methods: ['POST'])]
    public function forgotPassword(#[MapRequestPayload] ForgotPasswordRequest $payload, Request $request): JsonResponse
    {
        $this->requestLimiter->consumeEmailCode((string) $request->getClientIp());

        $user = $this->userRepository->findOneByEmail($this->normaliseEmail($payload->email));
        if ($user instanceof User && $user->isConfirmed() && null !== $user->getPassword()) {
            $this->emailCodeIssuer->issueAndSend($user, EmailCodePurpose::ResetPassword);
        }

        return new JsonResponse(new \stdClass(), Response::HTTP_ACCEPTED);
    }

    #[Route('/password/reset', methods: ['POST'])]
    public function resetPassword(#[MapRequestPayload] ResetPasswordRequest $payload, Request $request): Response
    {
        $this->requestLimiter->consumeEmailCode((string) $request->getClientIp());

        $user = $this->userRepository->findOneByEmail($this->normaliseEmail($payload->email));
        if (!$user instanceof User) {
            throw AccountException::invalidCode();
        }

        $this->emailCodeIssuer->verify($user, EmailCodePurpose::ResetPassword, $payload->code);

        $user->setPassword($this->passwordHasher->hashPassword($user, $payload->password));
        $this->deviceRepository->signOutAllExcept($user, $this->callingDeviceId($user), SignOutReason::PasswordChanged);
        $this->activityLog->record(ActivityKind::PasswordReset, $user, $this->callingDeviceId($user));
        $this->entityManager->flush();
        $this->accountMailer->sendPasswordChangedNotice($user);

        return new Response(status: Response::HTTP_NO_CONTENT);
    }

    #[Route('/password', methods: ['POST'])]
    #[IsGranted('ROLE_USER')]
    public function changePassword(#[MapRequestPayload] ChangePasswordRequest $payload, Request $request): Response
    {
        $this->requestLimiter->consumeLogin((string) $request->getClientIp());

        $user = $this->currentUser();
        if (null === $user->getPassword()) {
            throw AccountException::noPassword();
        }

        $this->checkCurrentPassword($user, $payload->currentPassword);

        $user->setPassword($this->passwordHasher->hashPassword($user, $payload->newPassword));
        $this->deviceRepository->signOutAllExcept($user, $this->currentDevice->get()->getDeviceId(), SignOutReason::PasswordChanged);
        $this->activityLog->record(ActivityKind::PasswordChanged, $user, $this->currentDevice->get()->getDeviceId());
        $this->entityManager->flush();
        $this->accountMailer->sendPasswordChangedNotice($user);

        return new Response(status: Response::HTTP_NO_CONTENT);
    }

    #[Route('', methods: ['DELETE'])]
    #[IsGranted('ROLE_USER')]
    public function delete(#[MapRequestPayload] DeleteAccountRequest $payload, Request $request): Response
    {
        $this->requestLimiter->consumeLogin((string) $request->getClientIp());

        $user = $this->currentUser();
        if (null === $user->getPassword()) {
            throw AccountException::noPassword();
        }

        $this->checkCurrentPassword($user, $payload->password);

        // Milestone 7.2 will also need to remove or reassign the member rows
        // this account owns in its libraries before libraries exist here.
        // The line keeps the account's id and the fingerprint of its email, not the email: it outlives the account (D66).
        $this->activityLog->record(ActivityKind::AccountDeleted, $user, $this->currentDevice->get()->getDeviceId());
        $this->entityManager->remove($user);
        $this->entityManager->flush();

        return new Response(status: Response::HTTP_NO_CONTENT);
    }

    #[Route('', methods: ['GET'])]
    #[IsGranted('ROLE_USER')]
    public function show(): JsonResponse
    {
        return new JsonResponse($this->accountPayload($this->currentUser()));
    }

    #[Route('', methods: ['PATCH'])]
    #[IsGranted('ROLE_USER')]
    public function update(#[MapRequestPayload] UpdateAccountRequest $payload): JsonResponse
    {
        $user = $this->currentUser();

        if (null !== $payload->name) {
            $user->setName($payload->name);
        }
        if (null !== $payload->locale) {
            $user->setLocale($payload->locale);
        }
        $this->entityManager->flush();

        return new JsonResponse($this->accountPayload($user));
    }

    #[Route('/devices', methods: ['GET'])]
    #[IsGranted('ROLE_USER')]
    public function devices(): JsonResponse
    {
        $currentDeviceId = $this->currentDevice->get()->getDeviceId();
        $devices = $this->deviceRepository->findBy(['user' => $this->currentUser()], ['createdAt' => 'ASC']);

        return new JsonResponse(array_map(
            static fn (Device $device): array => [
                'id' => $device->getDeviceId(),
                'name' => $device->getName(),
                'created_at' => $device->getCreatedAt()->format(\DateTimeInterface::ATOM),
                'last_seen_at' => $device->getLastSeenAt()?->format(\DateTimeInterface::ATOM),
                'signed_in' => $device->isSignedIn(),
                'removed' => $device->isRemoved(),
                'can_restore' => $device->canRestore(),
                'current' => $device->getDeviceId() === $currentDeviceId,
            ],
            $devices,
        ));
    }

    #[Route('/devices/{id}', methods: ['DELETE'])]
    #[IsGranted('ROLE_USER')]
    /**
     * Removes a device for good (D81), the calling one included: that is how
     * the app leaves an install to someone else. Removing it again is a no-op.
     */
    public function removeDevice(string $id): Response
    {
        $device = $this->deviceRepository->findOneOfUser($this->currentUser(), $id);
        if (!$device instanceof Device) {
            throw AccountException::deviceNotFound();
        }

        $now = new \DateTimeImmutable('now', new \DateTimeZone('UTC'));
        $device->remove($now);
        // The calling install removing itself already knows: nothing to undo.
        if ($device->getDeviceId() === $this->currentDevice->get()->getDeviceId()) {
            $device->noticeRemoval($now);
        }
        $this->entityManager->flush();

        return new Response(status: Response::HTTP_NO_CONTENT);
    }

    /**
     * Undoes a removal made by mistake (D81), only while the install has not
     * been told: once it has, it has wiped its libraries and started again.
     */
    #[Route('/devices/{id}/restore', methods: ['POST'])]
    #[IsGranted('ROLE_USER')]
    public function restoreDevice(string $id): Response
    {
        $device = $this->deviceRepository->findOneOfUser($this->currentUser(), $id);
        if (!$device instanceof Device) {
            throw AccountException::deviceNotFound();
        }

        if (!$device->isRemoved()) {
            return new Response(status: Response::HTTP_NO_CONTENT);
        }

        if (!$device->canRestore()) {
            throw AccountException::deviceAlreadyReset();
        }

        $device->restore();
        $this->entityManager->flush();

        return new Response(status: Response::HTTP_NO_CONTENT);
    }

    /** @return array{id: string, email: string, name: string, locale: string, has_password: bool, created_at: string, two_factor_enabled: bool, backup_codes_left: int} */
    private function accountPayload(User $user): array
    {
        return [
            'id' => $user->getId()->toRfc4122(),
            'email' => $user->getEmail(),
            'name' => $user->getName(),
            'locale' => $user->getLocale(),
            'has_password' => null !== $user->getPassword(),
            'created_at' => $user->getCreatedAt()->format(\DateTimeInterface::ATOM),
            'two_factor_enabled' => $user->isTwoFactorEnabled(),
            'backup_codes_left' => $this->backupCodeRepository->countUnusedByUser($user),
        ];
    }

    private function currentUser(): User
    {
        $user = $this->getUser();
        \assert($user instanceof User);

        return $user;
    }

    private function normaliseEmail(string $email): string
    {
        return mb_strtolower(trim($email));
    }

    /**
     * The device id of this request's own device token, kept signed in by
     * resetPassword(), but only when it belongs to the account being reset:
     * the route is public, so a token for a different account must not
     * protect anything of this one.
     */
    /**
     * The same cut per account as signing in (SEG-051): a session on a phone left unlocked must not be a way to try
     * passwords, since the password outlives the session.
     *
     * @throws AccountException with too_many_requests or invalid_credentials
     */
    private function checkCurrentPassword(User $user, string $password): void
    {
        $fingerprint = $this->fingerprints->of($user->getEmail());
        $this->requestLimiter->requireLoginOpen($fingerprint);
        if (!$this->passwordHasher->isPasswordValid($user, $password)) {
            $this->requestLimiter->noteLoginFailure($fingerprint);

            throw AccountException::invalidCredentials();
        }
    }

    private function callingDeviceId(User $user): ?string
    {
        $device = $this->currentDevice->tryGet();
        if (!$device instanceof Device || !$device->getUser()->getId()->equals($user->getId())) {
            return null;
        }

        return $device->getDeviceId();
    }
}
