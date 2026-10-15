<?php

declare(strict_types=1);

namespace App\Security;

use App\Entity\User;
use App\Enum\EmailCodePurpose;
use App\Sync\Invitation\EmailFingerprint;
use Symfony\Component\PasswordHasher\Hasher\UserPasswordHasherInterface;

/**
 * What the account asks for, besides the session, before something that changes how it is entered (D100): a session
 * lasts long, and a phone that is unlocked in other hands has one. Proof of who is asking is the password, or for an
 * account that has none (it came in with Google) a code sent to its email; then, if it has turned the second factor
 * on, a code of that. Phone lock (PIN, fingerprint) is deliberately not asked.
 */
final readonly class SensitiveAction
{
    public function __construct(
        private UserPasswordHasherInterface $passwordHasher,
        private RequestLimiter $requestLimiter,
        private EmailFingerprint $fingerprints,
        private EmailCodeIssuer $emailCodes,
        private TwoFactor $twoFactor,
    ) {
    }

    /**
     * The password, or the email code of an account with no password: never the one for the other (a code from the
     * email does not stand in for a password that exists). The code is looked at, not spent: `spendProof()` does that
     * once the action is done, so a slip in what follows does not cost another email.
     *
     * @throws AccountException with invalid_credentials, too_many_requests, invalid_code, code_expired or too_many_attempts
     */
    public function proveWhoAsks(User $user, ?string $password, ?string $emailCode): void
    {
        if (null !== $user->getPassword()) {
            $this->requirePassword($user, $password);

            return;
        }

        if (null === $emailCode) {
            throw AccountException::invalidCredentials();
        }

        $this->emailCodes->check($user, EmailCodePurpose::ConfirmAction, $emailCode);
    }

    /** For what only asks for the password when there is one: the second factor's own code is the proof for the rest. @throws AccountException */
    public function requirePasswordIfAny(User $user, ?string $password): void
    {
        if (null !== $user->getPassword()) {
            $this->requirePassword($user, $password);
        }
    }

    /** Takes the email code away once the action it was for is done. */
    public function spendProof(User $user): void
    {
        if (null === $user->getPassword()) {
            $this->emailCodes->spend($user, EmailCodePurpose::ConfirmAction);
        }
    }

    /**
     * @throws AccountException with two_factor_required or invalid_two_factor_code
     */
    public function requireSecondFactor(User $user, ?string $totpCode): void
    {
        if (!$user->isTwoFactorEnabled()) {
            return;
        }

        if (null === $totpCode || '' === trim($totpCode)) {
            throw AccountException::twoFactorRequired();
        }

        $this->twoFactor->verify($user, $totpCode);
    }

    /**
     * The same cut per account as signing in (SEG-051): a session on a phone left unlocked must not be a way to try
     * passwords, since the password outlives the session.
     *
     * @throws AccountException with too_many_requests or invalid_credentials
     */
    private function requirePassword(User $user, ?string $password): void
    {
        if (null === $password || '' === $password) {
            throw AccountException::invalidCredentials();
        }

        $fingerprint = $this->fingerprints->of($user->getEmail());
        $this->requestLimiter->requireLoginOpen($fingerprint);
        if (!$this->passwordHasher->isPasswordValid($user, $password)) {
            $this->requestLimiter->noteLoginFailure($fingerprint);

            throw AccountException::invalidCredentials();
        }
    }
}
