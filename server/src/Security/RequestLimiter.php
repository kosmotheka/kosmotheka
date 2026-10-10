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
     * An invitation sent by email: per account and day, so the server is not a way to mail strangers, and per
     * recipient and day, so one address cannot be flooded from several accounts. The recipient is a keyed
     * fingerprint, never the address. @throws AccountException with AccountErrorCode::TooManyRequests.
     */
    public function consumeInvitationEmail(string $userId, string $recipientFingerprint): void
    {
        $this->consume($this->invitationEmailLimiter, $userId);
        $this->consume($this->invitationRecipientLimiter, $recipientFingerprint);
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

    private function consume(RateLimiterFactoryInterface $factory, string $key): void
    {
        $limit = $factory->create($key)->consume();
        if (!$limit->isAccepted()) {
            throw AccountException::tooManyRequests($limit->getRetryAfter()->getTimestamp() - time());
        }
    }
}
