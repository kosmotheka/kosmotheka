<?php

declare(strict_types=1);

namespace App\Enum;

/** What a 6-digit email code is for (D49, D50). */
enum EmailCodePurpose: string
{
    case ConfirmEmail = 'confirm_email';
    case ResetPassword = 'reset_password';
    case ConfirmAction = 'confirm_action';
}
