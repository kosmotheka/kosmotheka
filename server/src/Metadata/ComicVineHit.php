<?php

declare(strict_types=1);

namespace App\Metadata;

use App\Enum\ComicVineResource;

/** What a Comic Vine search says of one result, a series (`volume`) or an issue: what needs no second call. */
final readonly class ComicVineHit
{
    public function __construct(
        /** Only {@see ComicVineResource::Volume} or {@see ComicVineResource::Issue}. */
        public ComicVineResource $resource,
        public int $id,
        public ?string $title,
        public ?string $series,
        public ?string $number,
        public ?int $year,
        public ?string $publisher,
        public ?string $coverUrl,
    ) {
    }

    /** @return array{resource: string, external_id: int, title: ?string, series: ?string, number: ?string, year: ?int, publisher: ?string, cover_url: ?string} */
    public function toArray(): array
    {
        return ['resource' => $this->resource->value, 'external_id' => $this->id, 'title' => $this->title, 'series' => $this->series, 'number' => $this->number, 'year' => $this->year, 'publisher' => $this->publisher, 'cover_url' => $this->coverUrl];
    }

    /** @param array<array-key, mixed> $data what {@see self::toArray()} gave, read back from the cache */
    public static function fromArray(array $data): ?self
    {
        $resource = \is_string($data['resource'] ?? null) ? ComicVineResource::tryFrom($data['resource']) : null;
        $id = $data['external_id'] ?? null;
        if (!\in_array($resource, [ComicVineResource::Volume, ComicVineResource::Issue], true) || !\is_int($id) || $id < 1) {
            return null;
        }

        return new self(
            $resource,
            $id,
            \is_string($data['title'] ?? null) ? $data['title'] : null,
            \is_string($data['series'] ?? null) ? $data['series'] : null,
            \is_string($data['number'] ?? null) ? $data['number'] : null,
            \is_int($data['year'] ?? null) ? $data['year'] : null,
            \is_string($data['publisher'] ?? null) ? $data['publisher'] : null,
            \is_string($data['cover_url'] ?? null) ? $data['cover_url'] : null,
        );
    }
}
