<?php

declare(strict_types=1);

namespace App\Security;

use App\Entity\EmailCode;
use App\Entity\User;
use App\Enum\EmailCodePurpose;
use App\Repository\EmailCodeRepository;
use Doctrine\ORM\EntityManagerInterface;

/**
 * Issues, sends and verifies the 6-digit codes (D49, D50): one live code per
 * user and purpose, 15 minutes, 5 attempts, single use.
 */
final readonly class EmailCodeIssuer
{
    private const int EXPIRY_MINUTES = 15;
    private const int MAX_ATTEMPTS = 5;

    public function __construct(
        private EntityManagerInterface $entityManager,
        private EmailCodeRepository $emailCodeRepository,
        private AccountMailer $accountMailer,
        private RequestLimiter $requestLimiter,
        #[\SensitiveParameter]
        private string $secret,
    ) {
        if ('' === $secret) {
            throw new \LogicException('APP_SECRET is empty: set it in .env.local (docs/development/server.md).');
        }
    }

    /** Issues a fresh code, replacing any live one for the same user and purpose, and emails it. */
    public function issueAndSend(User $user, EmailCodePurpose $purpose): void
    {
        // Spent before the live code is replaced: anyone may ask for a code for an address, and asking past its limit
        // must not take away the code its owner already has.
        $this->requestLimiter->consumeEmailPerAddress($user->getEmail());

        $existing = $this->emailCodeRepository->findOneByUserAndPurpose($user, $purpose);
        if ($existing instanceof EmailCode) {
            // Flushed on its own: Doctrine runs inserts before deletes in a
            // single flush, which would collide with uniq_email_code_user_purpose.
            $this->entityManager->remove($existing);
            $this->entityManager->flush();
        }

        $code = $this->generateCode();
        $expiresAt = new \DateTimeImmutable('now', new \DateTimeZone('UTC'))
            ->modify(\sprintf('+%d minutes', self::EXPIRY_MINUTES));

        $emailCode = new EmailCode($user, $purpose, $this->hash($code), $expiresAt);
        $this->entityManager->persist($emailCode);
        $this->entityManager->flush();

        match ($purpose) {
            EmailCodePurpose::ConfirmEmail => $this->accountMailer->sendConfirmationCode($user, $code, self::EXPIRY_MINUTES),
            EmailCodePurpose::ResetPassword => $this->accountMailer->sendPasswordResetCode($user, $code, self::EXPIRY_MINUTES),
            EmailCodePurpose::ConfirmAction => $this->accountMailer->sendActionCode($user, $code, self::EXPIRY_MINUTES),
        };
    }

    /**
     * Checks the code and spends it.
     *
     * @throws AccountException with invalid_code, code_expired or too_many_attempts
     */
    public function verify(User $user, EmailCodePurpose $purpose, string $code): void
    {
        $this->check($user, $purpose, $code);
        $this->spend($user, $purpose);
    }

    /** Takes a spent code away, once whatever it was for has been done. */
    public function spend(User $user, EmailCodePurpose $purpose): void
    {
        $emailCode = $this->emailCodeRepository->findOneByUserAndPurpose($user, $purpose);
        if ($emailCode instanceof EmailCode) {
            $this->entityManager->remove($emailCode);
            $this->entityManager->flush();
        }
    }

    /**
     * Whether the code is the live one, without spending it: for what also asks for something else that may be
     * wrong (the second factor), so that a slip there does not cost the person another email. A wrong code counts
     * as an attempt all the same.
     *
     * @throws AccountException with invalid_code, code_expired or too_many_attempts
     */
    public function check(User $user, EmailCodePurpose $purpose, string $code): void
    {
        $emailCode = $this->emailCodeRepository->findOneByUserAndPurpose($user, $purpose);
        if (!$emailCode instanceof EmailCode) {
            throw AccountException::invalidCode();
        }

        if ($emailCode->isExpired(new \DateTimeImmutable('now', new \DateTimeZone('UTC')))) {
            $this->entityManager->remove($emailCode);
            $this->entityManager->flush();

            throw AccountException::codeExpired();
        }

        if (hash_equals($emailCode->getCodeHash(), $this->hash($code))) {
            return;
        }

        $emailCode->registerFailedAttempt();
        if ($emailCode->getAttempts() >= self::MAX_ATTEMPTS) {
            $this->entityManager->remove($emailCode);
            $this->entityManager->flush();

            throw AccountException::tooManyAttempts();
        }

        $this->entityManager->flush();

        throw AccountException::invalidCode();
    }

    private function generateCode(): string
    {
        return \sprintf('%06d', random_int(0, 999_999));
    }

    private function hash(string $code): string
    {
        return hash_hmac('sha256', $code, $this->secret);
    }
}
