<?php

declare(strict_types=1);

namespace App\Controller;

use App\Dto\TwoFactorDisableRequest;
use App\Dto\TwoFactorEnableRequest;
use App\Dto\TwoFactorRegenerateBackupCodesRequest;
use App\Entity\User;
use App\Security\AccountException;
use App\Security\AccountMailer;
use App\Security\RequestLimiter;
use App\Security\TwoFactor;
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
 * Second factor (D78, milestone 7.1 part 4, docs/architecture/accounts.md):
 * setup, enabling, disabling and regenerating backup codes. Thin: maps, calls
 * App\Security\TwoFactor, returns JSON.
 */
#[Route('/v1/account/two-factor')]
#[IsGranted('ROLE_USER')]
final class TwoFactorController extends AbstractController
{
    public function __construct(
        private readonly TwoFactor $twoFactor,
        private readonly UserPasswordHasherInterface $passwordHasher,
        private readonly EntityManagerInterface $entityManager,
        private readonly RequestLimiter $requestLimiter,
        private readonly AccountMailer $accountMailer,
    ) {
    }

    /** @throws AccountException with two_factor_already_enabled or two_factor_unavailable */
    #[Route('/setup', methods: ['POST'])]
    public function setup(): JsonResponse
    {
        return new JsonResponse($this->twoFactor->startSetup($this->currentUser()));
    }

    #[Route('/enable', methods: ['POST'])]
    public function enable(#[MapRequestPayload] TwoFactorEnableRequest $payload, Request $request): JsonResponse
    {
        $this->requestLimiter->consumeLogin((string) $request->getClientIp());

        $user = $this->currentUser();
        $backupCodes = $this->twoFactor->enable($user, $payload->code);
        $this->accountMailer->sendTwoFactorEnabledNotice($user);

        return new JsonResponse(['backup_codes' => $backupCodes]);
    }

    #[Route('', methods: ['DELETE'])]
    public function disable(#[MapRequestPayload] TwoFactorDisableRequest $payload, Request $request): Response
    {
        $this->requestLimiter->consumeLogin((string) $request->getClientIp());

        $user = $this->currentUser();
        $this->checkPassword($user, $payload->password);
        $this->checkEnabled($user);
        $this->twoFactor->verify($user, $payload->code);

        $this->twoFactor->disable($user);
        $this->entityManager->flush();
        $this->accountMailer->sendTwoFactorDisabledNotice($user);

        return new Response(status: Response::HTTP_NO_CONTENT);
    }

    #[Route('/backup-codes', methods: ['POST'])]
    public function regenerateBackupCodes(#[MapRequestPayload] TwoFactorRegenerateBackupCodesRequest $payload, Request $request): JsonResponse
    {
        $this->requestLimiter->consumeLogin((string) $request->getClientIp());

        $user = $this->currentUser();
        $this->checkPassword($user, $payload->password);
        $this->checkEnabled($user);
        $this->twoFactor->verify($user, $payload->code);

        // regenerateBackupCodes() flushes itself, which also persists verify()'s anti-replay update above.
        $codes = $this->twoFactor->regenerateBackupCodes($user);

        return new JsonResponse(['backup_codes' => $codes]);
    }

    /** @throws AccountException with invalid_credentials or no_password */
    private function checkPassword(User $user, string $password): void
    {
        if (null === $user->getPassword()) {
            throw AccountException::noPassword();
        }

        if (!$this->passwordHasher->isPasswordValid($user, $password)) {
            throw AccountException::invalidCredentials();
        }
    }

    /** @throws AccountException with two_factor_not_enabled */
    private function checkEnabled(User $user): void
    {
        if (!$user->isTwoFactorEnabled()) {
            throw AccountException::twoFactorNotEnabled();
        }
    }

    private function currentUser(): User
    {
        $user = $this->getUser();
        \assert($user instanceof User);

        return $user;
    }
}
