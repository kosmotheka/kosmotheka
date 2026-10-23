<?php

declare(strict_types=1);

namespace App\Metadata;

/**
 * GameUPC (gameupc.com), as the server asks it for its accounts (D62): which BoardGameGeek game the barcode on a box
 * is, and, when a person tells us, which one it was. Behind an interface so the tests never reach the network. What
 * comes out is already checked.
 */
interface GameUpcSource
{
    public const string NAME = 'gameupc';

    /** Whether this server has the key it needs to ask. */
    public function isConfigured(): bool;

    /** @throws MetadataProviderException when it could not be asked, or said no */
    public function lookup(Barcode $code): GameUpcAnswer;

    /**
     * Tells GameUPC which game this barcode is, as a person chose it. `$userId` is what GameUPC keeps to undo the
     * choices of one bad user; it must say nothing about who that is ({@see GameUpcUserId}).
     *
     * @throws MetadataProviderException when it could not be told, or said no
     */
    public function choose(Barcode $code, int $gameId, string $userId): void;
}
