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
}
