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

    public static function deviceNotFound(): self
    {
        return new self(AccountErrorCode::DeviceNotFound, 404);
    }

    /** The account has no password yet (a future Google-only account). */
    public static function noPassword(): self
    {
        return new self(AccountErrorCode::NoPassword, 409);
    }
}
