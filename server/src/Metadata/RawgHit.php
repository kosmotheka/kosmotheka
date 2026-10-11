<?php

declare(strict_types=1);

namespace App\Metadata;

/** What a RAWG search says of one result: the part of a candidate that needs no second call. */
final readonly class RawgHit
{
    /** @param list<string> $platforms */
    public function __construct(
        public int $id,
        public ?string $title,
        public ?int $year,
        public ?string $coverUrl,
        public array $platforms,
    ) {
    }

    /** @return array{external_id: int, title: ?string, year: ?int, cover_url: ?string, platforms: list<string>} */
    public function toArray(): array
    {
        return ['external_id' => $this->id, 'title' => $this->title, 'year' => $this->year, 'cover_url' => $this->coverUrl, 'platforms' => $this->platforms];
    }

    /** @param array<array-key, mixed> $data what {@see self::toArray()} gave, read back from the cache */
    public static function fromArray(array $data): ?self
    {
        $id = $data['external_id'] ?? null;
        if (!\is_int($id) || $id < 1) {
            return null;
        }
        $platforms = [];
        foreach (\is_array($data['platforms'] ?? null) ? $data['platforms'] : [] as $platform) {
            if (\is_string($platform)) {
                $platforms[] = $platform;
            }
        }

        return new self(
            $id,
            \is_string($data['title'] ?? null) ? $data['title'] : null,
            \is_int($data['year'] ?? null) ? $data['year'] : null,
            \is_string($data['cover_url'] ?? null) ? $data['cover_url'] : null,
            $platforms,
        );
    }
}
