<?php

declare(strict_types=1);

namespace App\Metadata;

/** One game GameUPC names for a barcode: its BoardGameGeek number, its name and its cover. */
final readonly class GameUpcGame
{
    public const int MAX_ID = 1_000_000_000;

    public function __construct(
        public int $id,
        public string $title,
        public ?string $coverUrl,
    ) {
    }

    /** @return array{external_id: int, title: string, cover_url: ?string} */
    public function toArray(): array
    {
        return ['external_id' => $this->id, 'title' => $this->title, 'cover_url' => $this->coverUrl];
    }

    /** @param array<array-key, mixed> $data what {@see self::toArray()} gave, read back from the cache */
    public static function fromArray(array $data): ?self
    {
        $id = $data['external_id'] ?? null;
        $title = TextCleaner::whole($data['title'] ?? null, BggHttpSource::MAX_TEXT);
        if (!\is_int($id) || $id < 1 || $id > self::MAX_ID || null === $title) {
            return null;
        }

        return new self($id, $title, BggHttpSource::coverAddress($data['cover_url'] ?? null));
    }
}
