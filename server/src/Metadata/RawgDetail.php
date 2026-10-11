<?php

declare(strict_types=1);

namespace App\Metadata;

/** What the detail of a game adds to a search result: who made it and who published it. */
final readonly class RawgDetail
{
    /**
     * @param list<string> $developers
     * @param list<string> $publishers
     */
    public function __construct(
        public array $developers = [],
        public array $publishers = [],
    ) {
    }

    /** @return array{developers: list<string>, publishers: list<string>} */
    public function toArray(): array
    {
        return ['developers' => $this->developers, 'publishers' => $this->publishers];
    }

    /** @param array<array-key, mixed> $data what {@see self::toArray()} gave, read back from the cache */
    public static function fromArray(array $data): self
    {
        return new self(self::names($data['developers'] ?? null), self::names($data['publishers'] ?? null));
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
