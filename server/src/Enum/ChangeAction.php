<?php

declare(strict_types=1);

namespace App\Enum;

enum ChangeAction: string
{
    case Create = 'create';
    case Update = 'update';
    case Delete = 'delete';
}
