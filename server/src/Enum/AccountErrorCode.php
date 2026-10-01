<?php

declare(strict_types=1);

namespace App\Enum;

/**
 * Business error codes for the account API (docs/architecture/accounts.md).
 * App\Security\AccountException carries one of these; App\EventListener\
 * AccountExceptionListener turns it into the `{"error": "<code>"}` body.
 */
enum AccountErrorCode: string
{
    case InvalidCode = 'invalid_code';
    case CodeExpired = 'code_expired';
    case TooManyAttempts = 'too_many_attempts';
    case TooManyRequests = 'too_many_requests';
}
