<?php

declare(strict_types=1);

namespace App\Security\Google;

use App\Entity\GoogleAttempt;
use App\Entity\User;
use App\Repository\GoogleAttemptRepository;
use App\Security\AccountException;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;

/**
 * The life of one try at «Continuar con Google» (D77). It keeps no secret in the clear that could be used from
 * outside: the app's secret and the code are hashes, the state and the cookie that binds it to a browser are hashes,
 * and the PKCE secret never leaves the server.
 */
final readonly class GoogleAttempts
{
    /** How long an attempt lives, from the app's start to Google's return. */
    private const string ATTEMPT_LIFETIME = '+10 minutes';

    /** How long the one-use code lives, from Google's return to the app's exchange. */
    private const string CODE_LIFETIME = '+2 minutes';

    public function __construct(
        private GoogleAttemptRepository $attempts,
        private GoogleSignIn $google,
        private GoogleAccounts $accounts,
        private EntityManagerInterface $entityManager,
        private ClockInterface $clock,
    ) {
    }

    /** @return array{attempt: string, expiresAt: \DateTimeImmutable} */
    public function start(string $verifierHash, string $locale): array
    {
        $this->requireConfigured();
        $attempt = new GoogleAttempt($this->random(24), $verifierHash, $locale, $this->now()->modify(self::ATTEMPT_LIFETIME));
        $this->entityManager->persist($attempt);
        $this->entityManager->flush();

        return ['attempt' => $attempt->getId(), 'expiresAt' => $attempt->getExpiresAt()];
    }

    /**
     * Sends the browser to Google, once: the attempt must be fresh and not yet connected. The cookie value binds Google's
     * answer to this browser.
     *
     * @return ?array{url: string, binder: string} null when the attempt does not exist, expired or was already used
     */
    public function connect(string $attemptId): ?array
    {
        $this->requireConfigured();
        $attempt = $this->attempts->find($attemptId);
        if (!$attempt instanceof GoogleAttempt || GoogleAttempt::CREATED !== $attempt->getPhase() || $this->expired($attempt)) {
            return null;
        }

        $state = $this->random(32);
        $binder = $this->random(32);
        $authorization = $this->google->authorization($state);
        $attempt->connect($this->hash($state), $this->hash($binder), $authorization->pkceVerifier);
        $this->entityManager->flush();

        return ['url' => $authorization->url, 'binder' => $binder];
    }

    /**
     * Google answered the browser. The state has to be one this server gave, of an attempt still waiting for Google, and
     * the cookie has to be the one given to the same browser. Either way the attempt is closed here: a state is good once.
     *
     * @return ?array{attempt: string, code?: string, error?: string} null when the state is not valid (nobody to tell)
     */
    public function receive(string $state, string $binder, ?string $code, ?string $error): ?array
    {
        $this->requireConfigured();
        $attempt = '' === $state ? null : $this->attempts->findOneByStateHash($this->hash($state));
        if (!$attempt instanceof GoogleAttempt
            || GoogleAttempt::CONNECTED !== $attempt->getPhase()
            || $this->expired($attempt)
            || '' === $binder
            || !hash_equals((string) $attempt->getBinderHash(), $this->hash($binder))
        ) {
            return null;
        }

        $pkce = (string) $attempt->getPkceVerifier();
        $attempt->close();
        $this->entityManager->flush();

        if (null !== $error || null === $code || '' === $code) {
            return ['attempt' => $attempt->getId(), 'error' => 'access_denied' === $error ? 'google_cancelled' : 'google_failed'];
        }

        try {
            $user = $this->accounts->resolve($this->google->identify($code, $pkce), $attempt->getLocale());
        } catch (GoogleSignInException) {
            return ['attempt' => $attempt->getId(), 'error' => 'google_failed'];
        } catch (AccountException $exception) {
            return ['attempt' => $attempt->getId(), 'error' => $exception->errorCode->value];
        }

        $oneUse = $this->random(32);
        $attempt->grant($user, $this->hash($oneUse), $this->now()->modify(self::CODE_LIFETIME));
        $this->entityManager->flush();

        return ['attempt' => $attempt->getId(), 'code' => $oneUse];
    }

    /**
     * The app asks for its token: the code, with the secret the app kept. A wrong one never says which part was wrong, and
     * five wrong tries end the attempt. Nothing is spent until the sign-in is complete, so a missing second factor does
     * not burn the code; `done()` spends it.
     *
     * @throws AccountException with invalid_google_code
     */
    public function claim(string $attemptId, string $code, string $verifier): GoogleAttempt
    {
        $this->requireConfigured();
        $attempt = $this->attempts->find($attemptId);
        $codeHash = $attempt instanceof GoogleAttempt ? $attempt->getCodeHash() : null;
        if (!$attempt instanceof GoogleAttempt
            || GoogleAttempt::DONE !== $attempt->getPhase()
            || !$attempt->getUser() instanceof User
            || null === $codeHash
            || $this->expired($attempt)
            || $this->now() > $attempt->getCodeExpiresAt()
        ) {
            throw AccountException::invalidGoogleCode();
        }

        $rightCode = hash_equals($codeHash, $this->hash($code));
        $rightVerifier = hash_equals($attempt->getVerifierHash(), sodium_bin2base64(hash('sha256', $verifier, true), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING));
        if (!$rightCode || !$rightVerifier) {
            $attempt->noteFailure();
            if ($attempt->getFailures() >= GoogleAttempt::MAX_FAILURES) {
                $this->entityManager->remove($attempt);
            }
            $this->entityManager->flush();

            throw AccountException::invalidGoogleCode();
        }

        return $attempt;
    }

    /** The code is spent. */
    public function done(GoogleAttempt $attempt): void
    {
        $this->entityManager->remove($attempt);
    }

    public function isConfigured(): bool
    {
        return $this->google->isConfigured();
    }

    private function requireConfigured(): void
    {
        if (!$this->google->isConfigured()) {
            throw AccountException::googleUnavailable();
        }
    }

    private function expired(GoogleAttempt $attempt): bool
    {
        return $this->now() > $attempt->getExpiresAt();
    }

    private function now(): \DateTimeImmutable
    {
        return $this->clock->now()->setTimezone(new \DateTimeZone('UTC'));
    }

    /** @param positive-int $bytes */
    private function random(int $bytes): string
    {
        return sodium_bin2base64(random_bytes($bytes), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
    }

    private function hash(string $value): string
    {
        return hash('sha256', $value);
    }
}
