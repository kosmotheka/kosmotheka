<?php

declare(strict_types=1);

namespace App\Enum;

/** What a member may do in a library (D55, D73); the values are those of `CatalogEntity::ROLES`. */
enum Role: string
{
    case Admin = 'admin';
    case Editor = 'editor';
    case Reader = 'reader';
}
