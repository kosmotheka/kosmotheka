<?php

declare(strict_types=1);

namespace App\Enum;

use Symfony\Component\HttpFoundation\Response;

/** Why the server would not keep or give a cover (D82). The value is the `code` the app reads. */
enum CoverRejection: string
{
    /** No edition of this library points to it, or the server has none: the same answer, so that it reveals nothing. */
    case NotFound = 'cover_not_found';
    /** Its bytes do not give the hash in its name. */
    case NotTheCover = 'cover_not_the_one_named';
    /** Its bytes are not an image of the type its extension says. */
    case NotAnImage = 'cover_not_an_image';
    case TooLarge = 'cover_too_large';
    /** The library has no room left for covers. */
    case LibraryFull = 'covers_library_full';

    public function status(): int
    {
        return match ($this) {
            self::NotFound => Response::HTTP_NOT_FOUND,
            self::TooLarge, self::LibraryFull => Response::HTTP_REQUEST_ENTITY_TOO_LARGE,
            self::NotTheCover, self::NotAnImage => Response::HTTP_UNPROCESSABLE_ENTITY,
        };
    }
}
