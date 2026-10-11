<?php

declare(strict_types=1);

namespace App\Metadata;

/** The people of a series or an issue, by what they did: those who wrote it and those who drew it. */
final readonly class ComicVineDetail
{
    /**
     * @param list<string> $writers
     * @param list<string> $artists
     */
    public function __construct(
        public array $writers = [],
        public array $artists = [],
    ) {
    }

    /** @return array{writers: list<string>, artists: list<string>} */
    public function toArray(): array
    {
        return ['writers' => $this->writers, 'artists' => $this->artists];
    }

    /** @param array<array-key, mixed> $data what {@see self::toArray()} gave, read back from the cache */
    public static function fromArray(array $data): self
    {
        return new self(self::names($data['writers'] ?? null), self::names($data['artists'] ?? null));
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
