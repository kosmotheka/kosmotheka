<?php

declare(strict_types=1);

namespace App\Metadata;

/**
 * RAWG, as the server asks it for its accounts (7.8, D62), behind an interface so the tests never reach the network.
 * What comes out is already cut and checked: nothing of RAWG's own answer goes beyond these values.
 */
interface RawgSource
{
    public const string NAME = 'rawg';

    /** Whether this server has the key it needs to ask. */
    public function isConfigured(): bool;

    /**
     * @return list<RawgHit> at most eight, the first that are well formed
     *
     * @throws MetadataProviderException when it could not be asked, or said no
     */
    public function search(string $query): array;

    /**
     * The detail of results RAWG itself named, asked together. A detail that cannot be had is simply missing from the
     * answer: the search does not depend on it.
     *
     * @param list<int>        $ids
     * @param \Closure(): bool $mayCall asked before every real call; false = the day's cap is spent, skip it
     * @param \Closure(): void $onQuota called when RAWG refuses on quota
     *
     * @return array<int, RawgDetail|null> null when the provider answered that it has none, or nothing usable; missing when it could not be asked
     */
    public function details(array $ids, \Closure $mayCall, \Closure $onQuota): array;
}
