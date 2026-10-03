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
    case SignedOutPasswordChanged = 'signed_out_password_changed';
    case DeviceAlreadyReset = 'device_already_reset';
    case InvalidToken = 'invalid_token';
    case AuthenticationRequired = 'authentication_required';

    // Milestone 7.1, part 3: password recovery and change, deleting the account.
    case NoPassword = 'no_password';

    // Milestone 7.1, part 4: second factor (D78).
    case TwoFactorUnavailable = 'two_factor_unavailable';
    case TwoFactorAlreadyEnabled = 'two_factor_already_enabled';
    case TwoFactorNotStarted = 'two_factor_not_started';
    case TwoFactorNotEnabled = 'two_factor_not_enabled';
    case InvalidTwoFactorCode = 'invalid_two_factor_code';
    case TwoFactorRequired = 'two_factor_required';

    // Milestone 7.2: the cloud routes (docs/architecture/cloud.md).
    case NotFound = 'not_found';
    case SchemaVersion = 'schema_version';
    case InvalidRequest = 'invalid_request';
}
