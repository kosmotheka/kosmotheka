<?php

declare(strict_types=1);

namespace App\Enum;

/** The two kinds of invitation (D43, amended on 10/10/2026): the kind decides who may accept it and what it gives. */
enum InvitationKind: string
{
    /** Tied to an email: only an account with that verified email accepts it; a person already in the library keeps their role. */
    case Email = 'email';

    /** A link to copy: whoever opens it first enters, so it is only for a new person and always gives a reader. */
    case Link = 'link';
}
