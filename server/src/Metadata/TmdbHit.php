<?php

declare(strict_types=1);

namespace App\Metadata;

/** What a TMDB search says of one result: the part of a candidate that needs no second call. */
final readonly class TmdbHit
{
    public function __construct(
        public int $id,
        public ?string $title,
        public ?string $originalTitle,
        public ?int $year,
        public ?string $language,
        public ?string $coverUrl,
    ) {
    }

    /** @return array{external_id: int, title: ?string, original_title: ?string, year: ?int, language: ?string, cover_url: ?string} */
    public function toArray(): array
    {
        return ['external_id' => $this->id, 'title' => $this->title, 'original_title' => $this->originalTitle, 'year' => $this->year, 'language' => $this->language, 'cover_url' => $this->coverUrl];
    }

    /** @param array<array-key, mixed> $data what {@see self::toArray()} gave, read back from the cache */
    public static function fromArray(array $data): ?self
    {
        $id = $data['external_id'] ?? null;
        if (!\is_int($id) || $id < 1) {
            return null;
        }

        return new self(
            $id,
            \is_string($data['title'] ?? null) ? $data['title'] : null,
            \is_string($data['original_title'] ?? null) ? $data['original_title'] : null,
            \is_int($data['year'] ?? null) ? $data['year'] : null,
            \is_string($data['language'] ?? null) ? $data['language'] : null,
            \is_string($data['cover_url'] ?? null) ? $data['cover_url'] : null,
        );
    }
}
