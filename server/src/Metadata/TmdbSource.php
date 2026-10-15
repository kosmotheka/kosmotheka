<?php

declare(strict_types=1);

namespace App\Metadata;

use App\Enum\MediaKind;

/**
 * TMDB, as the server asks it for its accounts (7.8, D62), behind an interface so the tests never reach the network.
 * Not a {@see BookProvider}: that one looks a book up by ISBN, this one searches by title and then asks the detail of
 * each result. What comes out is already cut and checked: nothing of TMDB's own answer goes beyond these values.
 */
interface TmdbSource
{
    public const string NAME = 'tmdb';

    /** Whether this server has the token it needs to ask. */
    public function isConfigured(): bool;

    /**
     * @return list<TmdbHit> at most eight, the first that are well formed
     *
     * @throws MetadataProviderException when it could not be asked, or said no
     */
    public function search(MediaKind $kind, string $language, string $query): array;

    /**
     * The detail of results TMDB itself named, asked together. A detail that cannot be had is simply missing from the
     * answer: the search does not depend on it.
     *
     * @param list<int>        $ids
     * @param \Closure(): bool $mayCall asked before every real call; false = the day's cap is spent, skip it
     * @param \Closure(): void $onQuota called when TMDB refuses on quota
     *
     * @return array<int, TmdbDetail|null> null when the provider answered that it has none, or nothing usable; missing when it could not be asked
     */
    public function details(MediaKind $kind, string $language, array $ids, \Closure $mayCall, \Closure $onQuota): array;
}
