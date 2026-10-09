<?php

declare(strict_types=1);

namespace App\Enum;

/** What the permission rules made of an operation of the log. A stored operation not judged yet has no status. */
enum OperationStatus: string
{
    /** Allowed: its changes went through the merge. */
    case Applied = 'applied';
    /** Not allowed: it stays in the log as a request, and nothing of it is applied. */
    case Request = 'request';
}
