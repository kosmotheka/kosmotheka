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
use App\Security\AccountException;
use App\Security\AccountMailer;
use App\Security\ActivityLog;
use App\Security\CurrentDevice;
use App\Security\EmailCodeIssuer;
use App\Security\RegistrationGate;
use App\Security\RequestLimiter;
use App\Security\SensitiveAction;
use App\Sync\CloudOwnership;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\PasswordHasher\Hasher\PasswordHasherFactoryInterface;
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
        private readonly SensitiveAction $sensitiveAction,
        private readonly CloudOwnership $cloudOwnership,
        private readonly PasswordHasherFactoryInterface $hasherFactory,
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
    public function register(#[MapRequestPayload] RegisterAccountRequest $payload, Request $request): JsonResponse
    {
        $this->requestLimiter->consumeRegistration((string) $request->getClientIp());
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
    public function confirm(#[MapRequestPayload] ConfirmAccountRequest $payload, Request $request): Response
    {
        $this->requestLimiter->consumeEmailCode((string) $request->getClientIp());

        $user = $this->userRepository->findOneByEmail($this->normaliseEmail($payload->email));
        if (!$user instanceof User || $user->isConfirmed()) {
            throw AccountException::invalidCode();
        }

        $this->emailCodeIssuer->verify($user, EmailCodePurpose::ConfirmEmail, $payload->code);

        $user->setPassword($this->passwordHasher->hashPassword($user, $payload->password));
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
            $this->quietly(fn () => $this->emailCodeIssuer->issueAndSend($user, $payload->getPurpose()));
        }

        return $this->accepted();
    }

    /**
     * The code a signed-in account asks for to confirm something delicate when it has no password to give (D100). It
     * goes to its own address, so a limit on the address says nothing that is not already its own.
     */
    #[Route('/code/action', methods: ['POST'])]
    #[IsGranted('ROLE_USER')]
    public function requestActionCode(Request $request): JsonResponse
    {
        $this->requestLimiter->consumeEmailCode((string) $request->getClientIp());

        $this->emailCodeIssuer->issueAndSend($this->currentUser(), EmailCodePurpose::ConfirmAction);

        return $this->accepted();
    }

    #[Route('/password/forgot', methods: ['POST'])]
    public function forgotPassword(#[MapRequestPayload] ForgotPasswordRequest $payload, Request $request): JsonResponse
    {
        $this->requestLimiter->consumeEmailCode((string) $request->getClientIp());

        $user = $this->userRepository->findOneByEmail($this->normaliseEmail($payload->email));
        if ($user instanceof User && $user->isConfirmed() && null !== $user->getPassword()) {
            $this->quietly(fn () => $this->emailCodeIssuer->issueAndSend($user, EmailCodePurpose::ResetPassword));
        }

        return $this->accepted();
    }

    #[Route('/password/reset', methods: ['POST'])]
    public function resetPassword(#[MapRequestPayload] ResetPasswordRequest $payload, Request $request): Response
    {
        $this->requestLimiter->consumeEmailCode((string) $request->getClientIp());

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
    #[IsGranted('ROLE_USER')]
    public function changePassword(#[MapRequestPayload] ChangePasswordRequest $payload, Request $request): Response
    {
        $this->requestLimiter->consumeLogin((string) $request->getClientIp());

        $user = $this->currentUser();
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
    #[IsGranted('ROLE_USER')]
    public function delete(#[MapRequestPayload] DeleteAccountRequest $payload, Request $request): Response
    {
        $this->requestLimiter->consumeLogin((string) $request->getClientIp());

        $user = $this->currentUser();
        $this->sensitiveAction->proveWhoAsks($user, $payload->password, $payload->emailCode);
        $this->sensitiveAction->requireSecondFactor($user, $payload->totpCode);
        $this->sensitiveAction->spendProof($user);

        // One transaction: the libraries the account uploaded pass to someone else with a place in them, or go with it if
        // there is nobody (D99), and then the account goes. The line of the log keeps the account's id and the fingerprint
        // of its email, not the email: it outlives the account (D66).
        $deviceId = $this->currentDevice->get()->getDeviceId();
        $this->entityManager->wrapInTransaction(function () use ($user, $deviceId): void {
            $this->cloudOwnership->handOverLibrariesOf($user);
            $this->activityLog->record(ActivityKind::AccountDeleted, $user, $deviceId);
            $this->entityManager->remove($user);
        });

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
