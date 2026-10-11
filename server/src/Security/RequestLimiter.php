<?php

declare(strict_types=1);

namespace App\Security;

use Symfony\Component\DependencyInjection\Attribute\Target;
use Symfony\Component\RateLimiter\RateLimiterFactoryInterface;

/**
 * The single caller of the rate limiter factories (config/packages/
 * rate_limiter.yaml): controllers consume a limit through here, never a
 * factory directly, so every 429 looks the same (docs/architecture/accounts.md).
 */
final readonly class RequestLimiter
{
    public function __construct(
        #[Target('registration')]
        private RateLimiterFactoryInterface $registrationLimiter,
        #[Target('login')]
        private RateLimiterFactoryInterface $loginLimiter,
        #[Target('login_failed_account')]
        private RateLimiterFactoryInterface $loginFailedAccountLimiter,
        #[Target('google')]
        private RateLimiterFactoryInterface $googleLimiter,
        #[Target('metadata_account')]
        private RateLimiterFactoryInterface $metadataAccountLimiter,
        #[Target('google_books_calls')]
        private RateLimiterFactoryInterface $googleBooksCallsLimiter,
        #[Target('email_code')]
        private RateLimiterFactoryInterface $emailCodeLimiter,
        #[Target('email_per_address')]
        private RateLimiterFactoryInterface $emailPerAddressLimiter,
        #[Target('two_factor')]
        private RateLimiterFactoryInterface $twoFactorLimiter,
        #[Target('invitation_create')]
        private RateLimiterFactoryInterface $invitationCreateLimiter,
        #[Target('invitation_email')]
        private RateLimiterFactoryInterface $invitationEmailLimiter,
        #[Target('invitation_recipient_library')]
        private RateLimiterFactoryInterface $invitationRecipientLibraryLimiter,
        #[Target('invitation_recipient')]
        private RateLimiterFactoryInterface $invitationRecipientLimiter,
        #[Target('invitation_lookup')]
        private RateLimiterFactoryInterface $invitationLookupLimiter,
        #[Target('invitation_accept')]
        private RateLimiterFactoryInterface $invitationAcceptLimiter,
    ) {
    }

    /** @throws AccountException with AccountErrorCode::TooManyRequests */
    public function consumeRegistration(string $ip): void
    {
        $this->consume($this->registrationLimiter, $ip);
    }

    /** @throws AccountException with AccountErrorCode::TooManyRequests */
    public function consumeLogin(string $ip): void
    {
        $this->consume($this->loginLimiter, $ip);
    }

    /** Any step of «Continuar con Google», per address. @throws AccountException with AccountErrorCode::TooManyRequests */
    public function consumeGoogle(string $ip): void
    {
        $this->consume($this->googleLimiter, $ip);
    }

    /** Any lookup of a book through the server, per account, cache hits included (7.8). @throws AccountException with AccountErrorCode::TooManyRequests */
    public function consumeMetadata(string $userId): void
    {
        $this->consume($this->metadataAccountLimiter, $userId);
    }

    /**
     * One real call to Google Books, from the whole server: its free quota is a thousand a day and everybody shares it.
     * One key for all; nothing says whose call it was. @throws AccountException with AccountErrorCode::MetadataQuotaExhausted.
     */
    public function consumeGoogleBooksCall(): void
    {
        $limit = $this->googleBooksCallsLimiter->create('server')->consume();
        if (!$limit->isAccepted()) {
            throw AccountException::metadataQuotaExhausted($limit->getRetryAfter()->getTimestamp() - time());
        }
    }

    /**
     * Whether the account typed at a login has already failed too many times, without spending anything: a sign-in with
     * the right password is refused too, until the window passes (SEG-051). The key is the fingerprint of the address,
     * so an address with no account answers the same. @throws AccountException with AccountErrorCode::TooManyRequests.
     */
    public function requireLoginOpen(string $addressFingerprint): void
    {
        $this->requireOpen($this->loginFailedAccountLimiter, $addressFingerprint);
    }

    /** A failed sign-in (a wrong password or second-factor code) spends one try of that account; it never throws. */
    public function noteLoginFailure(string $addressFingerprint): void
    {
        $this->loginFailedAccountLimiter->create($addressFingerprint)->consume();
    }

    /** @throws AccountException with AccountErrorCode::TooManyRequests */
    public function consumeEmailCode(string $ip): void
    {
        $this->consume($this->emailCodeLimiter, $ip);
    }

    /** @throws AccountException with AccountErrorCode::TooManyRequests */
    public function consumeEmailPerAddress(string $normalisedEmail): void
    {
        $this->consume($this->emailPerAddressLimiter, $normalisedEmail);
    }

    /** @throws AccountException with AccountErrorCode::TooManyRequests */
    public function consumeTwoFactor(string $userId): void
    {
        $this->consume($this->twoFactorLimiter, $userId);
    }

    /** Making invitations, per account (D43). @throws AccountException with AccountErrorCode::TooManyRequests */
    public function consumeInvitationCreate(string $userId): void
    {
        $this->consume($this->invitationCreateLimiter, $userId);
    }

    /**
     * Whether an invitation by email may go now, spending nothing: per account and day (so the server is not a way to
     * mail strangers), per pair library and address (so another library cannot use up what this one may send to an
     * address) and per address in all (so none is flooded from many libraries). The recipient is a keyed fingerprint,
     * never the address. All three are looked at before any is spent (SEG-044).
     *
     * @throws AccountException with AccountErrorCode::TooManyRequests
     */
    public function requireInvitationEmailOpen(string $userId, string $libraryId, string $recipientFingerprint): void
    {
        $this->requireOpen($this->invitationEmailLimiter, $userId);
        $this->requireOpen($this->invitationRecipientLibraryLimiter, $libraryId.'|'.$recipientFingerprint);
        $this->requireOpen($this->invitationRecipientLimiter, $recipientFingerprint);
    }

    /** Spends the three of an invitation by email that was sent. Never throws: it is already gone. */
    public function spendInvitationEmail(string $userId, string $libraryId, string $recipientFingerprint): void
    {
        $this->invitationEmailLimiter->create($userId)->consume();
        $this->invitationRecipientLibraryLimiter->create($libraryId.'|'.$recipientFingerprint)->consume();
        $this->invitationRecipientLimiter->create($recipientFingerprint)->consume();
    }

    /** Looking at or accepting an invitation, per address: the token is long, but nobody should be able to try. @throws AccountException with AccountErrorCode::TooManyRequests */
    public function consumeInvitationLookup(string $ip): void
    {
        $this->consume($this->invitationLookupLimiter, $ip);
    }

    /** Accepting, per account, on top of the address. @throws AccountException with AccountErrorCode::TooManyRequests */
    public function consumeInvitationAccept(string $userId): void
    {
        $this->consume($this->invitationAcceptLimiter, $userId);
    }

    /** Looks at a limit without spending from it. */
    private function requireOpen(RateLimiterFactoryInterface $factory, string $key): void
    {
        $limit = $factory->create($key)->consume(0);
        if ($limit->getRemainingTokens() < 1) {
            throw AccountException::tooManyRequests($limit->getRetryAfter()->getTimestamp() - time());
        }
    }

    private function consume(RateLimiterFactoryInterface $factory, string $key): void
    {
        $limit = $factory->create($key)->consume();
        if (!$limit->isAccepted()) {
            throw AccountException::tooManyRequests($limit->getRetryAfter()->getTimestamp() - time());
        }
    }
}
