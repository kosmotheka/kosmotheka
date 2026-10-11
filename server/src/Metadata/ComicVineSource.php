<?php

declare(strict_types=1);

namespace App\Metadata;

use App\Enum\ComicVineResource;

/**
 * Comic Vine, as the server asks it for its accounts (7.8, D62), behind an interface so the tests never reach the
 * network. What comes out is already cut and checked: nothing of its own answer goes beyond these values.
 */
interface ComicVineSource
{
    public const string NAME = 'comic_vine';

    /** Whether this server has the key it needs to ask. */
    public function isConfigured(): bool;

    /**
     * Series and issues, nothing else it may answer: the series first, then the issues, eight at most.
     *
     * @return list<ComicVineHit>
     *
     * @throws MetadataProviderException when it could not be asked, or said no
     */
    public function search(string $query): array;

    /**
     * The people of results Comic Vine itself named, asked together, each by its own resource. A detail that cannot be
     * had is simply missing from the answer: the search does not depend on it.
     *
     * @param list<ComicVineHit>                $hits
     * @param \Closure(ComicVineResource): bool $mayCall asked before every real call; false = that resource's hour is spent, skip it
     * @param \Closure(): void                  $onQuota called when Comic Vine refuses on quota
     *
     * @return array<string, ComicVineDetail> by `volume:796` or `issue:6`
     */
    public function details(array $hits, \Closure $mayCall, \Closure $onQuota): array;
}
