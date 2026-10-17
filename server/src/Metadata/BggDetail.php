<?php

declare(strict_types=1);

namespace App\Metadata;

/** What the detail of a game or a book adds to a search result. Every text is as BoardGameGeek sent it, or missing. */
final readonly class BggDetail
{
    /**
     * @param list<string> $designers
     * @param list<string> $artists
     */
    public function __construct(
        public ?string $title = null,
        public ?string $coverUrl = null,
        public ?string $publisher = null,
        public array $designers = [],
        public array $artists = [],
        public ?int $playersMin = null,
        public ?int $playersMax = null,
        public ?int $durationMinutes = null,
        public ?int $minAge = null,
        /** The game this one is an expansion of: its number and its name, together or not at all. */
        public ?int $expandsId = null,
        public ?string $expandsTitle = null,
    ) {
    }

    /** @return array{title: ?string, cover_url: ?string, publisher: ?string, designers: list<string>, artists: list<string>, players_min: ?int, players_max: ?int, duration_minutes: ?int, min_age: ?int, expands: ?array{external_id: int, title: string}} */
    public function toArray(): array
    {
        return [
            'title' => $this->title,
            'cover_url' => $this->coverUrl,
            'publisher' => $this->publisher,
            'designers' => $this->designers,
            'artists' => $this->artists,
            'players_min' => $this->playersMin,
            'players_max' => $this->playersMax,
            'duration_minutes' => $this->durationMinutes,
            'min_age' => $this->minAge,
            'expands' => null !== $this->expandsId && null !== $this->expandsTitle ? ['external_id' => $this->expandsId, 'title' => $this->expandsTitle] : null,
        ];
    }

    /** @param array<array-key, mixed> $data what {@see self::toArray()} gave, read back from the cache: it passes the same checks again (SEG-057) */
    public static function fromArray(array $data): self
    {
        $expands = $data['expands'] ?? null;
        $expandsId = \is_array($expands) ? ($expands['external_id'] ?? null) : null;
        $expandsTitle = \is_array($expands) ? TextCleaner::whole($expands['title'] ?? null, BggHttpSource::MAX_TEXT) : null;
        $keepExpands = \is_int($expandsId) && $expandsId >= 1 && null !== $expandsTitle;

        return new self(
            TextCleaner::whole($data['title'] ?? null, BggHttpSource::MAX_TEXT),
            BggHttpSource::coverAddress($data['cover_url'] ?? null),
            TextCleaner::whole($data['publisher'] ?? null, BggHttpSource::MAX_TEXT),
            self::names($data['designers'] ?? null),
            self::names($data['artists'] ?? null),
            self::number($data['players_min'] ?? null, 99),
            self::number($data['players_max'] ?? null, 99),
            self::number($data['duration_minutes'] ?? null, BggHttpSource::MAX_MINUTES),
            self::number($data['min_age'] ?? null, 99),
            $keepExpands ? $expandsId : null,
            $keepExpands ? $expandsTitle : null,
        );
    }

    private static function number(mixed $value, int $max): ?int
    {
        return \is_int($value) && $value >= 1 && $value <= $max ? $value : null;
    }

    /** @return list<string> */
    private static function names(mixed $value): array
    {
        $names = [];
        foreach (\is_array($value) ? $value : [] as $name) {
            $clean = TextCleaner::whole($name, BggHttpSource::MAX_TEXT);
            if (null !== $clean && \count($names) < BggHttpSource::MAX_PEOPLE) {
                $names[] = $clean;
            }
        }

        return $names;
    }
}
