<?php

declare(strict_types=1);

namespace App\Enum;

/**
 * What a BoardGameGeek search is for (7.8, D62): board games, or role-playing books (RPGGeek's data). Both are asked on
 * boardgamegeek.com with the one token: tried on 17/10/2026, rpggeek.com answers a Cloudflare challenge instead of the
 * XML, and `rpgitem` works on boardgamegeek.com. A refusal of one scope pauses only that scope.
 */
enum BggScope: string
{
    case Boardgame = 'boardgame';
    case Rpg = 'rpg';

    /** The API address: the same for both scopes. */
    public function api(): string
    {
        return 'https://boardgamegeek.com/xmlapi2';
    }

    /** @return list<string> the item types that count as a result */
    public function itemTypes(): array
    {
        return match ($this) {
            self::Boardgame => ['boardgame', 'boardgameexpansion'],
            self::Rpg => ['rpgitem'],
        };
    }

    /** The name the pause and the cache keep this scope under: a refusal of one site never stops the other. */
    public function provider(): string
    {
        return 'bgg_'.$this->value;
    }

    public function designerLink(): string
    {
        return match ($this) {
            self::Boardgame => 'boardgamedesigner',
            self::Rpg => 'rpgdesigner',
        };
    }

    public function artistLink(): string
    {
        return match ($this) {
            self::Boardgame => 'boardgameartist',
            self::Rpg => 'rpgartist',
        };
    }

    public function publisherLink(): string
    {
        return match ($this) {
            self::Boardgame => 'boardgamepublisher',
            self::Rpg => 'rpgpublisher',
        };
    }
}
