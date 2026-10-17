<?php

declare(strict_types=1);

namespace App\Metadata;

use App\Enum\BggScope;

/**
 * BoardGameGeek and RPGGeek, as the server asks them for its accounts (7.8, D62), behind an interface so the tests never
 * reach the network. What comes out is already checked, and none of BoardGameGeek's values is cut: one that does not fit
 * is dropped whole, because its terms forbid changing its data.
 */
interface BggSource
{
    public const string NAME = 'bgg';

    /** Whether this server has the token it needs to ask. */
    public function isConfigured(): bool;

    /**
     * @return list<BggHit> at most eight, the first that are well formed
     *
     * @throws MetadataProviderException when it could not be asked, or said no
     */
    public function search(BggScope $scope, string $query): array;

    /**
     * The detail of results BoardGameGeek itself named, asked in one call. A detail that cannot be had is simply
     * missing from the answer: the search does not depend on it.
     *
     * @param list<int>        $ids
     * @param \Closure(): bool $mayCall asked before the real call; false = the day's cap is spent, skip it
     * @param \Closure(): void $onQuota called when BoardGameGeek refuses on quota
     *
     * @return array<int, BggDetail|null> null when it answered that it has none, or nothing usable; missing when it could not be asked
     */
    public function details(BggScope $scope, array $ids, \Closure $mayCall, \Closure $onQuota): array;
}
