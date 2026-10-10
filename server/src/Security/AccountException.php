<?php

declare(strict_types=1);

namespace App\Security;

use App\Enum\AccountErrorCode;

/**
 * Carries a stable error code for the account API. Caught in one place,
 * App\EventListener\AccountExceptionListener, and turned into the JSON body
 * `{"error": "<code>"}` with the matching HTTP status.
 */
final class AccountException extends \RuntimeException
{
    public function __construct(
        public readonly AccountErrorCode $errorCode,
        public readonly int $statusCode,
        public readonly ?int $retryAfterSeconds = null,
        /** @var array<string, mixed> more keys for the body, next to `error` */
        public readonly array $extra = [],
    ) {
        parent::__construct($errorCode->value);
    }

    public static function invalidCode(): self
    {
        return new self(AccountErrorCode::InvalidCode, 400);
    }

    public static function codeExpired(): self
    {
        return new self(AccountErrorCode::CodeExpired, 400);
    }

    public static function tooManyAttempts(): self
    {
        return new self(AccountErrorCode::TooManyAttempts, 400);
    }

    public static function tooManyRequests(int $retryAfterSeconds): self
    {
        return new self(AccountErrorCode::TooManyRequests, 429, $retryAfterSeconds);
    }

    public static function invalidChallenge(): self
    {
        return new self(AccountErrorCode::InvalidChallenge, 400);
    }

    public static function invalidDeviceProof(): self
    {
        return new self(AccountErrorCode::InvalidDeviceProof, 400);
    }

    public static function invalidCredentials(): self
    {
        return new self(AccountErrorCode::InvalidCredentials, 401);
    }

    public static function accountNotConfirmed(): self
    {
        return new self(AccountErrorCode::AccountNotConfirmed, 403);
    }

    public static function accountBlocked(): self
    {
        return new self(AccountErrorCode::AccountBlocked, 403);
    }

    public static function deviceTaken(): self
    {
        return new self(AccountErrorCode::DeviceTaken, 409);
    }

    public static function deviceKeyMismatch(): self
    {
        return new self(AccountErrorCode::DeviceKeyMismatch, 409);
    }

    public static function deviceRemoved(): self
    {
        return new self(AccountErrorCode::DeviceRemoved, 403);
    }

    public static function deviceAlreadyReset(): self
    {
        return new self(AccountErrorCode::DeviceAlreadyReset, 409);
    }

    public static function deviceNotFound(): self
    {
        return new self(AccountErrorCode::DeviceNotFound, 404);
    }

    /** The account has no password yet (a future Google-only account). */
    public static function noPassword(): self
    {
        return new self(AccountErrorCode::NoPassword, 409);
    }

    /** The server host has turned TWO_FACTOR_ENABLED off (D78). */
    public static function twoFactorUnavailable(): self
    {
        return new self(AccountErrorCode::TwoFactorUnavailable, 409);
    }

    /** This server runs without a Mercure hub (D18, D75). */
    public static function eventsUnavailable(): self
    {
        return new self(AccountErrorCode::EventsUnavailable, 409);
    }

    public static function twoFactorAlreadyEnabled(): self
    {
        return new self(AccountErrorCode::TwoFactorAlreadyEnabled, 409);
    }

    /** enable() called without a startSetup() first. */
    public static function twoFactorNotStarted(): self
    {
        return new self(AccountErrorCode::TwoFactorNotStarted, 409);
    }

    public static function twoFactorNotEnabled(): self
    {
        return new self(AccountErrorCode::TwoFactorNotEnabled, 409);
    }

    public static function invalidTwoFactorCode(): self
    {
        return new self(AccountErrorCode::InvalidTwoFactorCode, 400);
    }

    /** Login with 2FA enabled and no totp_code: the app must ask for a new challenge before retrying. */
    public static function twoFactorRequired(): self
    {
        return new self(AccountErrorCode::TwoFactorRequired, 401);
    }

    /**
     * The account, or this device, was cut from a library it had (D98, D71). The signed operations that withdrew the
     * device go with it, so the app checks them itself before it deletes anything.
     *
     * @param list<array{schemaVersion: int, operation: \stdClass}> $proofs
     */
    public static function accessWithdrawn(array $proofs): self
    {
        return new self(AccountErrorCode::AccessWithdrawn, 403, null, ['proofs' => $proofs]);
    }

    /** A cloud library the account cannot reach: the same answer as one that does not exist (D84). */
    public static function notFound(): self
    {
        return new self(AccountErrorCode::NotFound, 404);
    }

    /** The client and the server are on different database versions and must not sync (D31). */
    public static function schemaVersionMismatch(): self
    {
        return new self(AccountErrorCode::SchemaVersion, 409);
    }

    /** A body that is not JSON of the expected shape, where the Serializer cannot be used. */
    public static function invalidRequest(): self
    {
        return new self(AccountErrorCode::InvalidRequest, 400);
    }

    /** The link operation sent to the link route is not this library's own, or is not signed by a device of its owner (D93). */
    public static function linkRejected(): self
    {
        return new self(AccountErrorCode::LinkRejected, 422);
    }

    /** The log a push would leave says the library's own link was not allowed: the push is undone (D93). */
    public static function linkNotAllowed(): self
    {
        return new self(AccountErrorCode::LinkNotAllowed, 403);
    }

    /** The server has no address for invitation links (`INVITATION_BASE_URL`): it makes none (D18, D79). */
    public static function invitationsUnavailable(): self
    {
        return new self(AccountErrorCode::InvitationsUnavailable, 409);
    }

    /** The account is in the library but does not administer it: only whoever administers invites (D43). */
    public static function invitationForbidden(): self
    {
        return new self(AccountErrorCode::InvitationForbidden, 403);
    }

    /** The library has no persons and roles, or the cloud cannot link devices in it: the screen offers to turn them on (D43). */
    public static function invitationLibraryNotReady(): self
    {
        return new self(AccountErrorCode::InvitationLibraryNotReady, 409);
    }

    /** The person cannot be invited: not in this library, administers, has the access withdrawn, has an account, or (a link) is not a new reader. */
    public static function invitationPersonUnsuitable(): self
    {
        return new self(AccountErrorCode::InvitationPersonUnsuitable, 422);
    }

    public static function invitationExpired(): self
    {
        return new self(AccountErrorCode::InvitationExpired, 410);
    }

    public static function invitationUsed(): self
    {
        return new self(AccountErrorCode::InvitationUsed, 410);
    }

    /** Revoked by whoever administers, or no longer worth anything (the one who made it stopped administering, the person changed). */
    public static function invitationRevoked(): self
    {
        return new self(AccountErrorCode::InvitationRevoked, 410);
    }

    /** An invitation tied to an email, opened by an account with another one. */
    public static function invitationNotForThisAccount(): self
    {
        return new self(AccountErrorCode::InvitationNotForThisAccount, 403);
    }

    /** The account is already another person in that library: an invitation does not move it (D43). */
    public static function invitationAccountAlreadyInLibrary(): self
    {
        return new self(AccountErrorCode::InvitationAccountAlreadyInLibrary, 409);
    }
}
