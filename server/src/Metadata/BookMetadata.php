<?php

declare(strict_types=1);

namespace App\Metadata;

/**
 * What a provider knows about one edition: the same fields as the app's `Metadata`. The only shape that leaves the
 * server and the only one the cache keeps; nothing of the provider's own answer goes beyond it.
 */
final readonly class BookMetadata
{
    /** @param list<string> $authors */
    public function __construct(
        public ?string $title,
        public ?string $subtitle,
        public array $authors,
        public ?string $publisher,
        public ?int $publishedYear,
        public ?string $language,
        public ?string $coverUrl,
        public ?string $externalId,
    ) {
    }

    /** @return array{found: bool, title: ?string, subtitle: ?string, authors: list<string>, publisher: ?string, published_year: ?int, language: ?string, cover_url: ?string, external_id: ?string} */
    public static function notFoundPayload(): array
    {
        return ['found' => false, 'title' => null, 'subtitle' => null, 'authors' => [], 'publisher' => null, 'published_year' => null, 'language' => null, 'cover_url' => null, 'external_id' => null];
    }

    /** @return array{found: bool, title: ?string, subtitle: ?string, authors: list<string>, publisher: ?string, published_year: ?int, language: ?string, cover_url: ?string, external_id: ?string} */
    public function toArray(): array
    {
        return [
            'found' => true,
            'title' => $this->title,
            'subtitle' => $this->subtitle,
            'authors' => $this->authors,
            'publisher' => $this->publisher,
            'published_year' => $this->publishedYear,
            'language' => $this->language,
            'cover_url' => $this->coverUrl,
            'external_id' => $this->externalId,
        ];
    }
}
