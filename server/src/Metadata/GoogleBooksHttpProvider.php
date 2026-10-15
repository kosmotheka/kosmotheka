<?php

declare(strict_types=1);

namespace App\Metadata;

use Symfony\Component\DependencyInjection\Attribute\Autowire;
use Symfony\Contracts\HttpClient\HttpClientInterface;

/**
 * Google Books volumes API, with the server's key (D83). The address is fixed, the only input is an ISBN that already
 * passed `Isbn::parse`, and the key goes in a header, never in the address. What Google sends is read with a size cap,
 * cut and type-checked field by field: the app only ever receives `BookMetadata`.
 */
final readonly class GoogleBooksHttpProvider implements BookProvider
{
    public const string NAME = 'google_books';

    private const string URL = 'https://www.googleapis.com/books/v1/volumes';
    private const float TIMEOUT_SECONDS = 5.0;
    private const int MAX_BODY_BYTES = 512 * 1024;
    private const int MAX_TEXT = 300;
    private const int MAX_NAME = 200;
    private const int MAX_AUTHORS = 20;
    private const int MAX_URL = 500;
    /** Only this host serves the covers of Google Books; anything else a catalogue names is dropped. */
    private const string COVER_HOST = 'books.google.com';

    public function __construct(
        private HttpClientInterface $httpClient,
        #[Autowire(env: 'GOOGLE_BOOKS_API_KEY')]
        private string $apiKey,
    ) {
    }

    public function name(): string
    {
        return self::NAME;
    }

    public function isConfigured(): bool
    {
        return '' !== trim($this->apiKey);
    }

    public function lookup(Isbn $isbn): ?BookMetadata
    {
        $body = $this->fetch($isbn);

        return $this->normalise($body);
    }

    private function fetch(Isbn $isbn): string
    {
        try {
            $response = $this->httpClient->request('GET', self::URL, [
                'query' => ['q' => 'isbn:'.$isbn->value, 'maxResults' => 1, 'printType' => 'books'],
                'headers' => ['X-Goog-Api-Key' => trim($this->apiKey), 'Accept' => 'application/json'],
                'timeout' => self::TIMEOUT_SECONDS,
                'max_duration' => self::TIMEOUT_SECONDS,
                'max_redirects' => 0,
            ]);

            $status = $response->getStatusCode();
            if (429 === $status || 403 === $status) {
                $response->cancel();

                throw MetadataProviderException::quota();
            }
            if ($status >= 500) {
                $response->cancel();

                throw MetadataProviderException::brokeItself();
            }
            if (200 !== $status) {
                $response->cancel();

                throw MetadataProviderException::failed();
            }

            $body = '';
            foreach ($this->httpClient->stream($response) as $chunk) {
                $body .= $chunk->getContent();
                if (\strlen($body) > self::MAX_BODY_BYTES) {
                    $response->cancel();

                    throw MetadataProviderException::failed();
                }
            }

            return $body;
        } catch (MetadataProviderException $exception) {
            throw $exception;
        } catch (\Throwable) {
            // On purpose without `previous`: see MetadataProviderException.
            throw MetadataProviderException::failed();
        }
    }

    private function normalise(string $body): ?BookMetadata
    {
        $decoded = json_decode($body, true, 16);
        if (!\is_array($decoded)) {
            throw MetadataProviderException::failed();
        }

        $items = $decoded['items'] ?? null;
        $volume = \is_array($items) ? ($items[0] ?? null) : null;
        if (!\is_array($volume) || !\is_array($volume['volumeInfo'] ?? null)) {
            return null;
        }
        $info = $volume['volumeInfo'];

        $title = $this->text($info['title'] ?? null, self::MAX_TEXT);
        $authors = $this->authors($info['authors'] ?? null);
        // A volume with neither says nothing a form could use.
        if (null === $title && [] === $authors) {
            return null;
        }

        $images = $info['imageLinks'] ?? null;

        return new BookMetadata(
            title: $title,
            subtitle: $this->text($info['subtitle'] ?? null, self::MAX_TEXT),
            authors: $authors,
            publisher: $this->text($info['publisher'] ?? null, self::MAX_NAME),
            publishedYear: $this->year($info['publishedDate'] ?? null),
            language: $this->language($info['language'] ?? null),
            coverUrl: \is_array($images) ? self::cover($images['thumbnail'] ?? null) : null,
            externalId: $this->externalId($volume['id'] ?? null),
        );
    }

    private function text(mixed $value, int $max): ?string
    {
        return TextCleaner::clean($value, $max);
    }

    /** @return list<string> */
    private function authors(mixed $value): array
    {
        if (!\is_array($value)) {
            return [];
        }

        $authors = [];
        foreach (\array_slice($value, 0, 200) as $name) {
            $clean = $this->text($name, self::MAX_NAME);
            if (null !== $clean) {
                $authors[] = $clean;
            }
            if (\count($authors) >= self::MAX_AUTHORS) {
                break;
            }
        }

        return $authors;
    }

    private function year(mixed $value): ?int
    {
        if (!\is_string($value) || 1 !== preg_match('/^(\d{4})/', $value, $match)) {
            return null;
        }
        $year = (int) $match[1];

        return $year >= 1000 && $year <= 2100 ? $year : null;
    }

    private function language(mixed $value): ?string
    {
        return \is_string($value) && 1 === preg_match('/^[a-z]{2,3}(-[A-Za-z]{2,4})?$/D', $value) ? $value : null;
    }

    private function externalId(mixed $value): ?string
    {
        return \is_string($value) && 1 === preg_match('/^[A-Za-z0-9_-]{1,64}$/D', $value) ? $value : null;
    }

    /**
     * Google sends `http://` thumbnails with a page-curl effect (as the app knew): https, flat, and only from its own host.
     * What is read back from the cache passes it again (SEG-057).
     */
    public static function cover(mixed $value): ?string
    {
        if (!\is_string($value) || \strlen($value) > self::MAX_URL || 1 === preg_match('/[\x00-\x20\x7F]/', $value)) {
            return null;
        }
        $parts = parse_url($value);
        if (!\is_array($parts)
            || !\in_array($parts['scheme'] ?? '', ['http', 'https'], true)
            || self::COVER_HOST !== strtolower($parts['host'] ?? '')
            || isset($parts['user']) || isset($parts['pass']) || isset($parts['port'])
        ) {
            return null;
        }

        $query = array_filter(
            explode('&', $parts['query'] ?? ''),
            static fn (string $pair): bool => '' !== $pair && 'edge=curl' !== $pair,
        );

        return 'https://'.self::COVER_HOST.($parts['path'] ?? '/').([] === $query ? '' : '?'.implode('&', $query));
    }
}
