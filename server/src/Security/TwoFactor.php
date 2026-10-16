<?php

declare(strict_types=1);

namespace App\Security;

use App\Entity\BackupCode;
use App\Entity\User;
use App\Repository\BackupCodeRepository;
use App\Repository\UserRepository;
use Doctrine\ORM\EntityManagerInterface;
use OTPHP\TOTP;
use Symfony\Component\Clock\ClockInterface;
use Symfony\Component\DependencyInjection\Attribute\Autowire;

/**
 * Second factor (D78): TOTP plus ten backup codes, spomky-labs/otphp doing the
 * HOTP/TOTP maths. Deliberately not scheb/2fa-bundle, which relies on sessions
 * and a half-authenticated firewall state the stateless account API does not
 * have; the whole dance is driven by App\Controller\TwoFactorController and
 * the extra check in App\Controller\AuthController::login().
 */
final readonly class TwoFactor
{
    private const string ISSUER = 'Kosmotheka';
    private const int BACKUP_CODES_COUNT = 10;
    private const int BACKUP_CODE_LENGTH = 10;
    // No 0/O/1/I/L (D78): easy to misread off a screen or a printed card.
    private const string BACKUP_CODE_ALPHABET = '23456789ABCDEFGHJKMNPQRSTUVWXYZ';

    public function __construct(
        private EntityManagerInterface $entityManager,
        private BackupCodeRepository $backupCodeRepository,
        private UserRepository $userRepository,
        private RequestLimiter $requestLimiter,
        private ClockInterface $clock,
        private TwoFactorSecretBox $secretBox,
        #[Autowire(param: 'kernel.secret')]
        #[\SensitiveParameter]
        private string $secret,
        #[Autowire(env: 'bool:TWO_FACTOR_ENABLED')]
        private bool $enabled,
    ) {
    }

    public function isAvailable(): bool
    {
        return $this->enabled;
    }

    /**
     * @return array{secret: string, otpauth_uri: string}
     *
     * @throws AccountException with two_factor_already_enabled or two_factor_unavailable
     */
    public function startSetup(User $user): array
    {
        if ($user->isTwoFactorEnabled()) {
            throw AccountException::twoFactorAlreadyEnabled();
        }

        if (!$this->enabled) {
            throw AccountException::twoFactorUnavailable();
        }

        $totp = TOTP::generate($this->clock);
        $totp->setLabel($user->getUserIdentifier());
        $totp->setIssuer(self::ISSUER);

        $user->setTotpPendingSecret($this->secretBox->seal($user->getId()->toRfc4122(), $totp->getSecret()));
        $this->entityManager->flush();

        return ['secret' => $totp->getSecret(), 'otpauth_uri' => $totp->getProvisioningUri()];
    }

    /**
     * Confirms the pending secret and turns 2FA on, issuing the ten backup codes.
     *
     * @return list<string> the backup codes in clear, shown once
     *
     * @throws AccountException with two_factor_not_started or invalid_two_factor_code
     */
    public function enable(User $user, string $code): array
    {
        $pending = $user->getTotpPendingSecret();
        if (null === $pending) {
            throw AccountException::twoFactorNotStarted();
        }

        $this->requestLimiter->consumeTwoFactor($user->getId()->toRfc4122());
        try {
            $plain = $this->secretBox->open($user->getId()->toRfc4122(), $pending);
        } catch (TwoFactorSecretException) {
            throw AccountException::twoFactorNotStarted();
        }
        $timestep = $this->matchTimestep($this->totpFor($plain), trim($code), $user->getTotpLastTimestep());
        if (null === $timestep) {
            throw AccountException::invalidTwoFactorCode();
        }

        // Moved sealed as it is: it was sealed for this same account.
        $user->setTotpSecret($pending);
        $user->setTotpPendingSecret(null);
        $user->setTotpLastTimestep($timestep);
        $user->setTotpEnabledAt($this->clock->now());

        $codes = $this->replaceBackupCodes($user);
        $this->entityManager->flush();

        return $codes;
    }

    /**
     * Verifies a TOTP code (window ±1 step, anti-replay) or an unused backup
     * code, marking it used. Every attempt counts against a per-account limit,
     * because the login limit is per IP. The use is written at once, not left for
     * the caller's flush, so that two requests with one code cannot both pass (SEG-072).
     *
     * @throws AccountException with invalid_two_factor_code
     */
    public function verify(User $user, string $code): void
    {
        $this->requestLimiter->consumeTwoFactor($user->getId()->toRfc4122());
        $code = trim($code);
        $secret = $this->openSecret($user);
        if (null !== $secret) {
            $timestep = $this->matchTimestep($this->totpFor($secret), $code, $user->getTotpLastTimestep());
            if (null !== $timestep && $this->userRepository->claimTotpTimestep($user, $timestep)) {
                return;
            }
        }

        if ($this->consumeBackupCode($user, $code)) {
            return;
        }

        throw AccountException::invalidTwoFactorCode();
    }

    /** The account's TOTP secret in clear, or null when it has none or it does not open here (only the backup codes are left then). */
    private function openSecret(User $user): ?string
    {
        $sealed = $user->getTotpSecret();
        if (null === $sealed) {
            return null;
        }
        try {
            return $this->secretBox->open($user->getId()->toRfc4122(), $sealed);
        } catch (TwoFactorSecretException) {
            return null;
        }
    }

    public function disable(User $user): void
    {
        $user->setTotpSecret(null);
        $user->setTotpPendingSecret(null);
        $user->setTotpEnabledAt(null);
        $user->setTotpLastTimestep(null);
        $this->backupCodeRepository->deleteAllByUser($user);
    }

    /** @return list<string> the new backup codes in clear, shown once */
    public function regenerateBackupCodes(User $user): array
    {
        $codes = $this->replaceBackupCodes($user);
        $this->entityManager->flush();

        return $codes;
    }

    /** @return list<string> */
    private function replaceBackupCodes(User $user): array
    {
        $this->backupCodeRepository->deleteAllByUser($user);

        $codes = [];
        for ($i = 0; $i < self::BACKUP_CODES_COUNT; ++$i) {
            $plain = $this->generateBackupCode();
            $codes[] = $this->formatBackupCode($plain);
            $this->entityManager->persist(new BackupCode($user, $this->hashBackupCode($plain)));
        }

        return $codes;
    }

    private function consumeBackupCode(User $user, string $code): bool
    {
        $backupCode = $this->backupCodeRepository->findOneUnusedByUserAndHash($user, $this->hashBackupCode($code));
        if (!$backupCode instanceof BackupCode) {
            return false;
        }

        return $this->backupCodeRepository->markUsed($backupCode, $this->clock->now());
    }

    /**
     * The three timesteps a code may validly arrive at (±1 step), oldest
     * first, skipping any at or before `$lastTimestep` (anti-replay).
     */
    private function matchTimestep(TOTP $totp, string $code, ?int $lastTimestep): ?int
    {
        $period = $totp->getPeriod();
        $now = $this->clock->now()->getTimestamp();

        foreach ([-1, 0, 1] as $stepOffset) {
            $timestamp = $now + $stepOffset * $period;
            if ($timestamp < 0) {
                continue;
            }

            $timestep = intdiv($timestamp, $period);
            if (null !== $lastTimestep && $timestep <= $lastTimestep) {
                continue;
            }

            if (hash_equals($totp->at($timestamp), $code)) {
                return $timestep;
            }
        }

        return null;
    }

    private function totpFor(string $secret): TOTP
    {
        if ('' === $secret) {
            throw new \LogicException('A stored TOTP secret is never empty.');
        }

        return TOTP::createFromSecret($secret, $this->clock);
    }

    private function generateBackupCode(): string
    {
        $alphabetLength = \strlen(self::BACKUP_CODE_ALPHABET);
        $code = '';
        for ($i = 0; $i < self::BACKUP_CODE_LENGTH; ++$i) {
            $code .= self::BACKUP_CODE_ALPHABET[random_int(0, $alphabetLength - 1)];
        }

        return $code;
    }

    private function formatBackupCode(string $plain): string
    {
        return substr($plain, 0, 5).'-'.substr($plain, 5);
    }

    /** Accepted with or without the dash, case-insensitively (D78). */
    private function normaliseBackupCode(string $code): string
    {
        return mb_strtoupper(str_replace('-', '', trim($code)));
    }

    private function hashBackupCode(string $code): string
    {
        return hash_hmac('sha256', $this->normaliseBackupCode($code), $this->secret);
    }
}
