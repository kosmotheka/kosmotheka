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
use App\Enum\EmailCodePurpose;
use App\Repository\DeviceRepository;
use App\Repository\UserRepository;
use App\Security\AccountException;
use App\Security\AccountMailer;
use App\Security\CurrentDevice;
use App\Security\EmailCodeIssuer;
use App\Security\RequestLimiter;
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
    ) {
    }

    #[Route('/register', methods: ['POST'])]
    public function register(#[MapRequestPayload] RegisterAccountRequest $payload, Request $request): JsonResponse
    {
        $this->requestLimiter->consumeRegistration((string) $request->getClientIp());

        $email = $this->normaliseEmail($payload->email);
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
        $this->deviceRepository->signOutAllExcept($user, $this->callingDeviceId($user));
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

        if (!$this->passwordHasher->isPasswordValid($user, $payload->currentPassword)) {
            throw AccountException::invalidCredentials();
        }

        $user->setPassword($this->passwordHasher->hashPassword($user, $payload->newPassword));
        $this->deviceRepository->signOutAllExcept($user, $this->currentDevice->get()->getId());
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

        if (!$this->passwordHasher->isPasswordValid($user, $payload->password)) {
            throw AccountException::invalidCredentials();
        }

        // Milestone 7.2 will also need to remove or reassign the member rows
        // this account owns in its libraries before libraries exist here.
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
        $currentDeviceId = $this->currentDevice->get()->getId();
        $devices = $this->deviceRepository->findBy(['user' => $this->currentUser()], ['createdAt' => 'ASC']);

        return new JsonResponse(array_map(
            static fn (Device $device): array => [
                'id' => $device->getId(),
                'name' => $device->getName(),
                'created_at' => $device->getCreatedAt()->format(\DateTimeInterface::ATOM),
                'last_seen_at' => $device->getLastSeenAt()?->format(\DateTimeInterface::ATOM),
                'signed_in' => $device->isSignedIn(),
                'removed' => $device->isRemoved(),
                'can_restore' => $device->canRestore(),
                'current' => $device->getId() === $currentDeviceId,
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
        $device = $this->deviceRepository->find($id);
        if (!$device instanceof Device || !$device->getUser()->getId()->equals($this->currentUser()->getId())) {
            throw AccountException::deviceNotFound();
        }

        $now = new \DateTimeImmutable('now', new \DateTimeZone('UTC'));
        $device->remove($now);
        // The calling install removing itself already knows: nothing to undo.
        if ($device->getId() === $this->currentDevice->get()->getId()) {
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
        $device = $this->deviceRepository->find($id);
        if (!$device instanceof Device || !$device->getUser()->getId()->equals($this->currentUser()->getId())) {
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

    /** @return array{id: string, email: string, name: string, locale: string, has_password: bool, created_at: string} */
    private function accountPayload(User $user): array
    {
        return [
            'id' => $user->getId()->toRfc4122(),
            'email' => $user->getEmail(),
            'name' => $user->getName(),
            'locale' => $user->getLocale(),
            'has_password' => null !== $user->getPassword(),
            'created_at' => $user->getCreatedAt()->format(\DateTimeInterface::ATOM),
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
    private function callingDeviceId(User $user): ?string
    {
        $device = $this->currentDevice->tryGet();
        if (!$device instanceof Device || !$device->getUser()->getId()->equals($user->getId())) {
            return null;
        }

        return $device->getId();
    }
}
