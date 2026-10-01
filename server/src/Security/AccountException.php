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
}
