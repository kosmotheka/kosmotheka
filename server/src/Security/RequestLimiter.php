<?php

declare(strict_types=1);

namespace App\Security;

use App\Enum\ComicVineResource;
use App\Sync\Invitation\EmailFingerprint;
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
        #[Target('tmdb_calls')]
        private RateLimiterFactoryInterface $tmdbCallsLimiter,
        #[Target('rawg_calls')]
        private RateLimiterFactoryInterface $rawgCallsLimiter,
        #[Target('comic_vine_search')]
        private RateLimiterFactoryInterface $comicVineSearchLimiter,
        #[Target('comic_vine_volume')]
        private RateLimiterFactoryInterface $comicVineVolumeLimiter,
        #[Target('comic_vine_issue')]
        private RateLimiterFactoryInterface $comicVineIssueLimiter,
        #[Target('google_books_account')]
        private RateLimiterFactoryInterface $googleBooksAccountLimiter,
        #[Target('tmdb_account')]
        private RateLimiterFactoryInterface $tmdbAccountLimiter,
        #[Target('rawg_account')]
        private RateLimiterFactoryInterface $rawgAccountLimiter,
        #[Target('comic_vine_account')]
        private RateLimiterFactoryInterface $comicVineAccountLimiter,
        #[Target('cloud_sync')]
        private RateLimiterFactoryInterface $cloudSyncLimiter,
        #[Target('cloud_link')]
        private RateLimiterFactoryInterface $cloudLinkLimiter,
        #[Target('cloud_events')]
        private RateLimiterFactoryInterface $cloudEventsLimiter,
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
        private EmailFingerprint $fingerprints,
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
     * One real call to TMDB, from the whole server: the day's cap that keeps anybody from making the server hammer it.
     * One key for all; nothing says whose call it was. @throws AccountException with AccountErrorCode::MetadataQuotaExhausted.
     */
    public function consumeTmdbCall(): void
    {
        $limit = $this->tmdbCallsLimiter->create('server')->consume();
        if (!$limit->isAccepted()) {
            throw AccountException::metadataQuotaExhausted($limit->getRetryAfter()->getTimestamp() - time());
        }
    }

    /**
     * One real call to RAWG, from the whole server: its free quota is 20,000 a month, and a day gets its share. One
     * key for all; nothing says whose call it was. @throws AccountException with AccountErrorCode::MetadataQuotaExhausted.
     */
    public function consumeRawgCall(): void
    {
        $limit = $this->rawgCallsLimiter->create('server')->consume();
        if (!$limit->isAccepted()) {
            throw AccountException::metadataQuotaExhausted($limit->getRetryAfter()->getTimestamp() - time());
        }
    }

    /**
     * One real call to Comic Vine, from the whole server, against the hour of the resource it is made to: it supports
     * 200 an hour per resource and the server keeps below that. One key for all; nothing says whose call it was.
     *
     * @throws AccountException with AccountErrorCode::MetadataQuotaExhausted
     */
    public function consumeComicVineCall(ComicVineResource $resource): void
    {
        $factory = match ($resource) {
            ComicVineResource::Search => $this->comicVineSearchLimiter,
            ComicVineResource::Volume => $this->comicVineVolumeLimiter,
            ComicVineResource::Issue => $this->comicVineIssueLimiter,
        };
        $limit = $factory->create('server')->consume();
        if (!$limit->isAccepted()) {
            throw AccountException::metadataQuotaExhausted($limit->getRetryAfter()->getTimestamp() - time());
        }
    }

    /**
     * One real call to Google Books for this account: its own share of the day first, so that no account can spend
     * the server's for everybody (SEG-052), then the server's. What comes from the cache spends neither.
     *
     * @throws AccountException with AccountErrorCode::TooManyRequests or MetadataQuotaExhausted
     */
    public function consumeGoogleBooksCallOf(string $accountId): void
    {
        $this->consume($this->googleBooksAccountLimiter, $accountId);
        $this->consumeGoogleBooksCall();
    }

    /** As `consumeGoogleBooksCallOf`, for TMDB. @throws AccountException with AccountErrorCode::TooManyRequests or MetadataQuotaExhausted */
    public function consumeTmdbCallOf(string $accountId): void
    {
        $this->consume($this->tmdbAccountLimiter, $accountId);
        $this->consumeTmdbCall();
    }

    /** As `consumeGoogleBooksCallOf`, for RAWG. @throws AccountException with AccountErrorCode::TooManyRequests or MetadataQuotaExhausted */
    public function consumeRawgCallOf(string $accountId): void
    {
        $this->consume($this->rawgAccountLimiter, $accountId);
        $this->consumeRawgCall();
    }

    /** As `consumeGoogleBooksCallOf`, for Comic Vine, whose share is per resource and hour. @throws AccountException with AccountErrorCode::TooManyRequests or MetadataQuotaExhausted */
    public function consumeComicVineCallOf(string $accountId, ComicVineResource $resource): void
    {
        $this->consume($this->comicVineAccountLimiter, $accountId.'|'.$resource->value);
        $this->consumeComicVineCall($resource);
    }

    /** One upload to a cloud library, per account. @throws AccountException with AccountErrorCode::TooManyRequests */
    public function consumeCloudPush(string $userId): void
    {
        $this->consume($this->cloudSyncLimiter, $userId.'|push');
    }

    /** One download from a cloud library, per account. @throws AccountException with AccountErrorCode::TooManyRequests */
    public function consumeCloudPull(string $userId): void
    {
        $this->consume($this->cloudSyncLimiter, $userId.'|pull');
    }

    /** Claiming a person or linking a library, per account. @throws AccountException with AccountErrorCode::TooManyRequests */
    public function consumeCloudLink(string $userId): void
    {
        $this->consume($this->cloudLinkLimiter, $userId);
    }

    /** The token of the live notices, per account. @throws AccountException with AccountErrorCode::TooManyRequests */
    public function consumeCloudEvents(string $userId): void
    {
        $this->consume($this->cloudEventsLimiter, $userId);
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

    /**
     * Keyed by the fingerprint of the address, like the other limits on an address: the limiter's store never holds
     * the address itself (D66).
     *
     * @throws AccountException with AccountErrorCode::TooManyRequests
     */
    public function consumeEmailPerAddress(string $email): void
    {
        $this->consume($this->emailPerAddressLimiter, $this->fingerprints->of($email));
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
