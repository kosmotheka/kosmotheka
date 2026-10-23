<?php

declare(strict_types=1);

namespace App\Metadata;

/**
 * What GameUPC knows of one barcode, in the three cases it asks apps to handle: `verified` (one game, checked by
 * people), `choose` (its guesses, for the person to pick from or search instead) and `unknown` (nothing: the person
 * searches). Only a verified answer is a sure one.
 */
final readonly class GameUpcAnswer
{
    public const string VERIFIED = 'verified';
    public const string CHOOSE = 'choose';
    public const string UNKNOWN = 'unknown';
    public const int MAX_GAMES = 8;

    /** @param list<GameUpcGame> $games one when verified, none when unknown */
    private function __construct(
        public string $status,
        public array $games,
    ) {
    }

    public static function verified(GameUpcGame $game): self
    {
        return new self(self::VERIFIED, [$game]);
    }

    /** @param list<GameUpcGame> $games */
    public static function choose(array $games): self
    {
        $games = \array_slice($games, 0, self::MAX_GAMES);

        return [] === $games ? self::unknown() : new self(self::CHOOSE, $games);
    }

    public static function unknown(): self
    {
        return new self(self::UNKNOWN, []);
    }

    /** @return array{status: string, candidates: list<array{external_id: int, title: string, cover_url: ?string}>} */
    public function toArray(): array
    {
        return ['status' => $this->status, 'candidates' => array_map(static fn (GameUpcGame $game): array => $game->toArray(), $this->games)];
    }

    /** @param array<array-key, mixed> $data what {@see self::toArray()} gave, read back from the cache */
    public static function fromArray(array $data): self
    {
        $games = [];
        $stored = $data['candidates'] ?? [];
        foreach (\is_array($stored) ? $stored : [] as $row) {
            $game = \is_array($row) ? GameUpcGame::fromArray($row) : null;
            if ($game instanceof GameUpcGame) {
                $games[] = $game;
            }
        }

        return match ($data['status'] ?? null) {
            self::VERIFIED => [] !== $games ? self::verified($games[0]) : self::unknown(),
            self::CHOOSE => self::choose($games),
            default => self::unknown(),
        };
    }
}
