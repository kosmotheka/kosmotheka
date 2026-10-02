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

    // Milestone 7.1, part 2: device sign-in and the authenticated API.
    case InvalidChallenge = 'invalid_challenge';
    case InvalidDeviceProof = 'invalid_device_proof';
    case InvalidCredentials = 'invalid_credentials';
    case AccountNotConfirmed = 'account_not_confirmed';
    case AccountBlocked = 'account_blocked';
    case DeviceTaken = 'device_taken';
    case DeviceKeyMismatch = 'device_key_mismatch';
    case DeviceNotFound = 'device_not_found';
    case DeviceRemoved = 'device_removed';
    case InvalidToken = 'invalid_token';
    case AuthenticationRequired = 'authentication_required';

    // Milestone 7.1, part 3: password recovery and change, deleting the account.
    case NoPassword = 'no_password';
}
