<?php

declare(strict_types=1);

namespace App\Enum;

/**
 * What Comic Vine counts its requests by: it supports 200 an hour **per resource**, so the server keeps one limit for
 * each of the three it calls (7.8, D62).
 */
enum ComicVineResource: string
{
    case Search = 'search';
    case Volume = 'volume';
    case Issue = 'issue';
}
