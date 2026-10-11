<?php

declare(strict_types=1);

namespace App\Enum;

/** What a title search is for (7.8): the two kinds TMDB knows, with the words the app and TMDB use for them. */
enum MediaKind: string
{
    case Film = 'film';
    case TvSeries = 'tv_series';

    /** The part of TMDB's addresses that names the kind. */
    public function tmdbPath(): string
    {
        return match ($this) {
            self::Film => 'movie',
            self::TvSeries => 'tv',
        };
    }
}
