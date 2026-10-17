<?php

declare(strict_types=1);

namespace App\Metadata;

/** What a BoardGameGeek search says of one result: the part of a candidate that needs no second call. */
final readonly class BggHit
{
    public function __construct(
        public int $id,
        public string $title,
        public ?int $year,
    ) {
    }

    /** @return array{external_id: int, title: string, year: ?int} */
    public function toArray(): array
    {
        return ['external_id' => $this->id, 'title' => $this->title, 'year' => $this->year];
    }

    /** @param array<array-key, mixed> $data what {@see self::toArray()} gave, read back from the cache */
    public static function fromArray(array $data): ?self
    {
        $id = $data['external_id'] ?? null;
        $title = TextCleaner::whole($data['title'] ?? null, BggHttpSource::MAX_TEXT);
        if (!\is_int($id) || $id < 1 || null === $title) {
            return null;
        }
        $year = $data['year'] ?? null;

        return new self($id, $title, \is_int($year) && $year >= 1 && $year <= 2200 ? $year : null);
    }
}
