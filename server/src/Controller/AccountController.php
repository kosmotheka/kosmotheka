<?php

declare(strict_types=1);

namespace App\Controller;

use App\Dto\ConfirmAccountRequest;
use App\Dto\RegisterAccountRequest;
use App\Dto\RequestEmailCodeRequest;
use App\Entity\User;
use App\Enum\EmailCodePurpose;
use App\Repository\UserRepository;
use App\Security\AccountException;
use App\Security\AccountMailer;
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

/** Account creation and email confirmation (D49). Thin: maps, calls services, returns JSON. */
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

    private function normaliseEmail(string $email): string
    {
        return mb_strtolower(trim($email));
    }
}
