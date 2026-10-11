<?php

declare(strict_types=1);

namespace App\Metadata;

/**
 * A catalogue the server asks on behalf of its accounts (7.8, D83), behind an interface so the tests never reach the
 * network. The answer is already the fixed shape: whatever the catalogue sends is cut and checked before it gets here.
 */
interface BookProvider
{
    /** The id in the route, the cache and `GET /v1/server`'s `metadata` list. */
    public function name(): string;

    /** Whether this server has what it needs to ask (a key). */
    public function isConfigured(): bool;

    /**
     * @return ?BookMetadata null when the catalogue does not know the book
     *
     * @throws MetadataProviderException when it could not be asked, or said no
     */
    public function lookup(Isbn $isbn): ?BookMetadata;
}
