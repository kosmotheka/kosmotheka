<?php

declare(strict_types=1);

namespace App\Enum;

/**
 * What the activity log can say happened (D66). A closed list: a new kind is a new case here, and the place that
 * records it, never free text from a client.
 */
enum ActivityKind: string
{
    // The account.
    case AccountRegistered = 'account_registered';
    case LoginSucceeded = 'login_succeeded';
    case LoginFailed = 'login_failed';
    case DeviceSignedOut = 'device_signed_out';
    case PasswordChanged = 'password_changed';
    case PasswordReset = 'password_reset';
    case TwoFactorEnabled = 'two_factor_enabled';
    case TwoFactorDisabled = 'two_factor_disabled';
    case AccountDeleted = 'account_deleted';
    case AccountBlocked = 'account_blocked';
    case AccountUnblocked = 'account_unblocked';

    // The libraries: operations that reached the server by `push` and were applied.
    case LibraryUploaded = 'library_uploaded';
    case LibraryDeleted = 'library_deleted';
    case RoleChanged = 'role_changed';
    case CanShareChanged = 'can_share_changed';
    case DeviceAssigned = 'device_assigned';
    case DeviceAccessRevoked = 'device_access_revoked';
    case AccessWithdrawn = 'access_withdrawn';
    case AccessRestored = 'access_restored';
    case CloudPowerOn = 'cloud_power_on';
    case CloudPowerOff = 'cloud_power_off';

    // The invitations (D43): the kind of invitation, never the email or the token.
    case InvitationCreated = 'invitation_created';
    case InvitationRevoked = 'invitation_revoked';
    case InvitationAccepted = 'invitation_accepted';

    // What the cloud does by itself (D76).
    case AccountLinked = 'account_linked';
    case DeviceLinked = 'device_linked';
}
