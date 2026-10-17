<?php

declare(strict_types=1);

namespace App\Controller;

use App\Dto\TwoFactorDisableRequest;
use App\Dto\TwoFactorEnableRequest;
use App\Dto\TwoFactorRegenerateBackupCodesRequest;
use App\Entity\User;
use App\Enum\ActivityKind;
use App\Security\AccountException;
use App\Security\AccountMailer;
use App\Security\ActivityLog;
use App\Security\CurrentDevice;
use App\Security\RateLimitKey;
use App\Security\SensitiveAction;
use App\Security\TwoFactor;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\HttpKernel\Attribute\RateLimit;
use Symfony\Component\Routing\Attribute\Route;
use Symfony\Component\Security\Http\Attribute\CurrentUser;

/**
 * Second factor (D78, milestone 7.1 part 4, docs/architecture/accounts.md):
 * setup, enabling, disabling and regenerating backup codes. Thin: maps, calls
 * App\Security\TwoFactor, returns JSON.
 */
#[Route('/v1/account/two-factor')]
final class TwoFactorController extends AbstractController
{
    public function __construct(
        private readonly TwoFactor $twoFactor,
        private readonly SensitiveAction $sensitiveAction,
        private readonly EntityManagerInterface $entityManager,
        private readonly AccountMailer $accountMailer,
        private readonly ActivityLog $activityLog,
        private readonly CurrentDevice $currentDevice,
    ) {
    }

    /** @throws AccountException with two_factor_already_enabled or two_factor_unavailable */
    #[Route('/setup', methods: ['POST'])]
    public function setup(#[CurrentUser] User $user): JsonResponse
    {
        return new JsonResponse($this->twoFactor->startSetup($user));
    }

    #[Route('/enable', methods: ['POST'])]
    #[RateLimit('login', key: RateLimitKey::ip(...))]
    public function enable(#[MapRequestPayload] TwoFactorEnableRequest $payload, #[CurrentUser] User $user): JsonResponse
    {
        // A session alone does not turn it on (D100): the password, or for an account with none a code from its email.
        $this->sensitiveAction->proveWhoAsks($user, $payload->password, $payload->emailCode);
        $backupCodes = $this->twoFactor->enable($user, $payload->code);
        $this->sensitiveAction->spendProof($user);
        // `enable` has flushed: its own line goes in a flush of its own, and a failure here is an error that is logged, not silence.
        $this->activityLog->record(ActivityKind::TwoFactorEnabled, $user, $this->currentDevice->get()->getDeviceId());
        $this->entityManager->flush();
        $this->accountMailer->sendTwoFactorEnabledNotice($user);

        return new JsonResponse(['backup_codes' => $backupCodes]);
    }

    #[Route('', methods: ['DELETE'])]
    #[RateLimit('login', key: RateLimitKey::ip(...))]
    public function disable(#[MapRequestPayload] TwoFactorDisableRequest $payload, #[CurrentUser] User $user): Response
    {
        $this->sensitiveAction->requirePasswordIfAny($user, $payload->password);
        $this->checkEnabled($user);
        $this->twoFactor->verify($user, $payload->code);

        $this->twoFactor->disable($user);
        $this->activityLog->record(ActivityKind::TwoFactorDisabled, $user, $this->currentDevice->get()->getDeviceId());
        $this->entityManager->flush();
        $this->accountMailer->sendTwoFactorDisabledNotice($user);

        return new Response(status: Response::HTTP_NO_CONTENT);
    }

    #[Route('/backup-codes', methods: ['POST'])]
    #[RateLimit('login', key: RateLimitKey::ip(...))]
    public function regenerateBackupCodes(#[MapRequestPayload] TwoFactorRegenerateBackupCodesRequest $payload, #[CurrentUser] User $user): JsonResponse
    {
        $this->sensitiveAction->requirePasswordIfAny($user, $payload->password);
        $this->checkEnabled($user);
        $this->twoFactor->verify($user, $payload->code);

        // regenerateBackupCodes() flushes itself, which also persists verify()'s anti-replay update above.
        $codes = $this->twoFactor->regenerateBackupCodes($user);

        return new JsonResponse(['backup_codes' => $codes]);
    }

    /** @throws AccountException with two_factor_not_enabled */
    private function checkEnabled(User $user): void
    {
        if (!$user->isTwoFactorEnabled()) {
            throw AccountException::twoFactorNotEnabled();
        }
    }
}
