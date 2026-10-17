<?php

declare(strict_types=1);

namespace App\Security;

use App\Enum\ComicVineResource;
use App\Sync\Invitation\EmailFingerprint;
use Symfony\Component\DependencyInjection\Attribute\Target;
use Symfony\Component\RateLimiter\RateLimiterFactoryInterface;

/**
 * The limits that depend on the flow and cannot be an attribute (config/packages/rate_limiter.yaml): the failures of
 * an account, the mails of an address or an invitation, the second factor, the real calls to the providers. The ones
 * a request spends just by arriving go in a `#[RateLimit]` on the action (`RateLimitKey` gives the key). Both end in
 * the same 429 body (`AccountExceptionListener`, docs/architecture/accounts.md).
 */
final readonly class RequestLimiter
{
    public function __construct(
        #[Target('login_failed_account')]
        private RateLimiterFactoryInterface $loginFailedAccountLimiter,
        #[Target('google_books_calls')]
        private RateLimiterFactoryInterface $googleBooksCallsLimiter,
        #[Target('tmdb_calls')]
        private RateLimiterFactoryInterface $tmdbCallsLimiter,
        #[Target('rawg_calls')]
        private RateLimiterFactoryInterface $rawgCallsLimiter,
        #[Target('bgg_calls')]
        private RateLimiterFactoryInterface $bggCallsLimiter,
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
        #[Target('bgg_account')]
        private RateLimiterFactoryInterface $bggAccountLimiter,
        #[Target('comic_vine_account')]
        private RateLimiterFactoryInterface $comicVineAccountLimiter,
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
        private EmailFingerprint $fingerprints,
    ) {
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
     * One real call to BoardGameGeek or RPGGeek (they share one token), from the whole server: BoardGameGeek gives no
     * figure, so the day's cap is the server's own (`BGG_DAILY_CALL_CAP`). One key for all; nothing says whose call it was.
     *
     * @throws AccountException with AccountErrorCode::MetadataQuotaExhausted
     */
    public function consumeBggCall(): void
    {
        $limit = $this->bggCallsLimiter->create('server')->consume();
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

    /** As `consumeGoogleBooksCallOf`, for BoardGameGeek and RPGGeek. @throws AccountException with AccountErrorCode::TooManyRequests or MetadataQuotaExhausted */
    public function consumeBggCallOf(string $accountId): void
    {
        $this->consume($this->bggAccountLimiter, $accountId);
        $this->consumeBggCall();
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
