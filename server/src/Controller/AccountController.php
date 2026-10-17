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
use App\Enum\AccountErrorCode;
use App\Enum\ActivityKind;
use App\Enum\EmailCodePurpose;
use App\Enum\SignOutReason;
use App\Repository\BackupCodeRepository;
use App\Repository\DeviceRepository;
use App\Repository\UserRepository;
use App\Security\AccountDeletion;
use App\Security\AccountException;
use App\Security\AccountMailer;
use App\Security\ActivityLog;
use App\Security\CurrentDevice;
use App\Security\EmailCodeIssuer;
use App\Security\RateLimitKey;
use App\Security\RegistrationGate;
use App\Security\SensitiveAction;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\Clock\ClockInterface;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\HttpKernel\Attribute\RateLimit;
use Symfony\Component\PasswordHasher\Hasher\PasswordHasherFactoryInterface;
use Symfony\Component\PasswordHasher\Hasher\UserPasswordHasherInterface;
use Symfony\Component\Routing\Attribute\Route;
use Symfony\Component\Security\Http\Attribute\CurrentUser;

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
        private readonly CurrentDevice $currentDevice,
        private readonly DeviceRepository $deviceRepository,
        private readonly BackupCodeRepository $backupCodeRepository,
        private readonly ActivityLog $activityLog,
        private readonly RegistrationGate $registrationGate,
        private readonly SensitiveAction $sensitiveAction,
        private readonly AccountDeletion $accountDeletion,
        private readonly PasswordHasherFactoryInterface $hasherFactory,
        private readonly ClockInterface $clock,
    ) {
    }

    /**
     * Always 202 (D100): the answer says nothing of whether the address has an account, is on the list of a closed
     * server, or has spent its mails. The password of a new account is only a placeholder that lets signing in say
     * "confirm your email": it is chosen again with the code, in `confirm()`, so the one that stays is always the one
     * of whoever has the email; a second registration of the same address touches nothing, it only sends a new code. The hash is made on
     * every path so that none is quicker than the others.
     */
    #[Route('/register', methods: ['POST'])]
    #[RateLimit('registration', key: RateLimitKey::ip(...))]
    public function register(#[MapRequestPayload] RegisterAccountRequest $payload): JsonResponse
    {
        $passwordHash = $this->hasherFactory->getPasswordHasher(User::class)->hash($payload->password);

        $email = $this->normaliseEmail($payload->email);
        // Before anything is created or sent: a server that only lets some addresses in leaves nothing behind for the rest.
        if (!$this->registrationGate->allows($email)) {
            return $this->accepted();
        }
        $user = $this->userRepository->findOneByEmail($email);

        if (!$user instanceof User) {
            $user = new User($email, $payload->name, $payload->locale);
            $user->setPassword($passwordHash);
            $this->entityManager->persist($user);
            $this->entityManager->flush();
            $this->quietly(fn () => $this->emailCodeIssuer->issueAndSend($user, EmailCodePurpose::ConfirmEmail));

            return $this->accepted();
        }

        if (!$user->isConfirmed()) {
            // Nothing of the account changes: not the password, not the name, not the language. A second registration
            // of the same address kills the first one's code, so whoever comes last would be the one whose data
            // stays. The password is chosen with the code.
            $this->quietly(fn () => $this->emailCodeIssuer->issueAndSend($user, EmailCodePurpose::ConfirmEmail));

            return $this->accepted();
        }

        $this->quietly(fn () => $this->accountMailer->sendExistingAccountNotice($user));

        return $this->accepted();
    }

    #[Route('/confirm', methods: ['POST'])]
    #[RateLimit('email_code', key: RateLimitKey::ip(...))]
    public function confirm(#[MapRequestPayload] ConfirmAccountRequest $payload): Response
    {
        $user = $this->userRepository->findOneByEmail($this->normaliseEmail($payload->email));
        if (!$user instanceof User || $user->isConfirmed()) {
            throw AccountException::invalidCode();
        }

        $this->emailCodeIssuer->verify($user, EmailCodePurpose::ConfirmEmail, $payload->code);

        $user->setPassword($this->passwordHasher->hashPassword($user, $payload->password));
        $user->confirm($this->clock->now()->setTimezone(new \DateTimeZone('UTC')));
        $this->activityLog->record(ActivityKind::AccountRegistered, $user);
        $this->entityManager->flush();

        return new Response(status: Response::HTTP_NO_CONTENT);
    }

    #[Route('/code', methods: ['POST'])]
    #[RateLimit('email_code', key: RateLimitKey::ip(...))]
    public function requestCode(#[MapRequestPayload] RequestEmailCodeRequest $payload): JsonResponse
    {
        $user = $this->userRepository->findOneByEmail($this->normaliseEmail($payload->email));
        if ($user instanceof User && !$user->isConfirmed()) {
            $this->quietly(fn () => $this->emailCodeIssuer->issueAndSend($user, $payload->getPurpose()));
        }

        return $this->accepted();
    }

    /**
     * The code a signed-in account asks for to confirm something delicate when it has no password to give (D100). It
     * goes to its own address, so a limit on the address says nothing that is not already its own.
     */
    #[Route('/code/action', methods: ['POST'])]
    #[RateLimit('email_code', key: RateLimitKey::ip(...))]
    public function requestActionCode(#[CurrentUser] User $user): JsonResponse
    {
        $this->emailCodeIssuer->issueAndSend($user, EmailCodePurpose::ConfirmAction);

        return $this->accepted();
    }

    #[Route('/password/forgot', methods: ['POST'])]
    #[RateLimit('email_code', key: RateLimitKey::ip(...))]
    public function forgotPassword(#[MapRequestPayload] ForgotPasswordRequest $payload): JsonResponse
    {
        $user = $this->userRepository->findOneByEmail($this->normaliseEmail($payload->email));
        if ($user instanceof User && $user->isConfirmed() && null !== $user->getPassword()) {
            $this->quietly(fn () => $this->emailCodeIssuer->issueAndSend($user, EmailCodePurpose::ResetPassword));
        }

        return $this->accepted();
    }

    #[Route('/password/reset', methods: ['POST'])]
    #[RateLimit('email_code', key: RateLimitKey::ip(...))]
    public function resetPassword(#[MapRequestPayload] ResetPasswordRequest $payload): Response
    {
        $user = $this->userRepository->findOneByEmail($this->normaliseEmail($payload->email));
        if (!$user instanceof User) {
            throw AccountException::invalidCode();
        }

        // Looked at, not spent, until the second factor is also right: the email alone is not enough for an account that
        // has it (D100), and asking for the code again for a slip there would cost another email.
        $this->emailCodeIssuer->check($user, EmailCodePurpose::ResetPassword, $payload->code);
        $this->sensitiveAction->requireSecondFactor($user, $payload->totpCode);
        $this->emailCodeIssuer->spend($user, EmailCodePurpose::ResetPassword);

        $user->setPassword($this->passwordHasher->hashPassword($user, $payload->password));
        $this->deviceRepository->signOutAllExcept($user, $this->callingDeviceId($user), SignOutReason::PasswordChanged);
        $this->activityLog->record(ActivityKind::PasswordReset, $user, $this->callingDeviceId($user));
        $this->entityManager->flush();
        $this->accountMailer->sendPasswordChangedNotice($user);

        return new Response(status: Response::HTTP_NO_CONTENT);
    }

    #[Route('/password', methods: ['POST'])]
    #[RateLimit('login', key: RateLimitKey::ip(...))]
    public function changePassword(#[MapRequestPayload] ChangePasswordRequest $payload, #[CurrentUser] User $user): Response
    {
        if (null === $user->getPassword()) {
            throw AccountException::noPassword();
        }

        $this->sensitiveAction->proveWhoAsks($user, $payload->currentPassword, null);
        $this->sensitiveAction->requireSecondFactor($user, $payload->totpCode);

        $user->setPassword($this->passwordHasher->hashPassword($user, $payload->newPassword));
        $this->deviceRepository->signOutAllExcept($user, $this->currentDevice->get()->getDeviceId(), SignOutReason::PasswordChanged);
        $this->activityLog->record(ActivityKind::PasswordChanged, $user, $this->currentDevice->get()->getDeviceId());
        $this->entityManager->flush();
        $this->accountMailer->sendPasswordChangedNotice($user);

        return new Response(status: Response::HTTP_NO_CONTENT);
    }

    #[Route('', methods: ['DELETE'])]
    #[RateLimit('login', key: RateLimitKey::ip(...))]
    public function delete(#[MapRequestPayload] DeleteAccountRequest $payload, #[CurrentUser] User $user): Response
    {
        $this->sensitiveAction->proveWhoAsks($user, $payload->password, $payload->emailCode);
        $this->sensitiveAction->requireSecondFactor($user, $payload->totpCode);
        $this->sensitiveAction->spendProof($user);

        $this->accountDeletion->delete($user, $this->currentDevice->get()->getDeviceId());

        return new Response(status: Response::HTTP_NO_CONTENT);
    }

    #[Route('', methods: ['GET'])]
    public function show(#[CurrentUser] User $user): JsonResponse
    {
        return new JsonResponse($this->accountPayload($user));
    }

    #[Route('', methods: ['PATCH'])]
    public function update(#[MapRequestPayload] UpdateAccountRequest $payload, #[CurrentUser] User $user): JsonResponse
    {
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
    public function devices(#[CurrentUser] User $user): JsonResponse
    {
        $currentDeviceId = $this->currentDevice->get()->getDeviceId();
        $devices = $this->deviceRepository->findBy(['user' => $user], ['createdAt' => 'ASC']);

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

    /**
     * Removes a device for good (D81), the calling one included: that is how
     * the app leaves an install to someone else. Removing it again is a no-op.
     */
    #[Route('/devices/{id}', methods: ['DELETE'])]
    public function removeDevice(string $id, #[CurrentUser] User $user): Response
    {
        $device = $this->deviceRepository->findOneOfUser($user, $id);
        if (!$device instanceof Device) {
            throw AccountException::deviceNotFound();
        }

        $now = $this->clock->now()->setTimezone(new \DateTimeZone('UTC'));
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
    public function restoreDevice(string $id, #[CurrentUser] User $user): Response
    {
        $device = $this->deviceRepository->findOneOfUser($user, $id);
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

    private function normaliseEmail(string $email): string
    {
        return mb_strtolower(trim($email));
    }

    /** The same answer whatever was or was not done (D100). */
    private function accepted(): JsonResponse
    {
        return new JsonResponse(new \stdClass(), Response::HTTP_ACCEPTED);
    }

    /**
     * Sends a code or a notice for an address, and says nothing when its mails are spent: the answer must not tell an
     * address with an account from one without (D100). Nothing is sent nor changed then, which `EmailCodeIssuer`
     * guarantees by spending the limit before it touches a code. The limit by IP is not this one and still answers 429.
     */
    private function quietly(\Closure $send): void
    {
        try {
            $send();
        } catch (AccountException $exception) {
            if (AccountErrorCode::TooManyRequests !== $exception->errorCode) {
                throw $exception;
            }
        }
    }

    /**
     * The device id of this request's own device token, kept signed in by
     * resetPassword(), but only when it belongs to the account being reset:
     * the route is public, so a token for a different account must not
     * protect anything of this one.
     */
    private function callingDeviceId(User $user): ?string
    {
        $device = $this->currentDevice->tryGet();
        if (!$device instanceof Device || !$device->getUser()->getId()->equals($user->getId())) {
            return null;
        }

        return $device->getDeviceId();
    }
}
