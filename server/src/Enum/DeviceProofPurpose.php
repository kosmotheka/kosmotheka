<?php

declare(strict_types=1);

namespace App\Enum;

/** What a device's signature is for (D100): it is part of what is signed, so one made to sign in one way is no use for the other. */
enum DeviceProofPurpose: string
{
    case Login = 'login';
    case Google = 'google';
}
