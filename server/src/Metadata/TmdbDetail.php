<?php

declare(strict_types=1);

namespace App\Metadata;

/** What the detail of a film or a series adds to a search result: who made it and how long or how big it is. */
final readonly class TmdbDetail
{
    /**
     * @param list<string> $directors
     * @param list<string> $creators
     */
    public function __construct(
        public array $directors = [],
        public array $creators = [],
        public ?int $durationMinutes = null,
        public ?int $seasons = null,
    ) {
    }

    /** @return array{directors: list<string>, creators: list<string>, duration_minutes: ?int, seasons: ?int} */
    public function toArray(): array
    {
        return ['directors' => $this->directors, 'creators' => $this->creators, 'duration_minutes' => $this->durationMinutes, 'seasons' => $this->seasons];
    }

    /** @param array<array-key, mixed> $data what {@see self::toArray()} gave, read back from the cache */
    public static function fromArray(array $data): self
    {
        return new self(
            self::names($data['directors'] ?? null),
            self::names($data['creators'] ?? null),
            \is_int($data['duration_minutes'] ?? null) ? $data['duration_minutes'] : null,
            \is_int($data['seasons'] ?? null) ? $data['seasons'] : null,
        );
    }

    /** @return list<string> */
    private static function names(mixed $value): array
    {
        $names = [];
        foreach (\is_array($value) ? $value : [] as $name) {
            if (\is_string($name)) {
                $names[] = $name;
            }
        }

        return $names;
    }
}
