<?php

declare(strict_types=1);

namespace App\Tests\Support;

use Symfony\Component\HttpClient\MockHttpClient;
use Symfony\Component\HttpClient\Response\MockResponse;

/**
 * The Google Books API, in memory: answers what a test queued (in order) and remembers every request it was asked.
 * Static, because the test client boots a new kernel (and so a new http client) for every request.
 */
final class FakeGoogleBooksApi
{
    /** @var list<MockResponse|\Throwable> */
    private static array $queue = [];

    /** @var list<array{method: string, url: string, headers: array<string, list<string>>}> */
    public static array $requests = [];

    public static function reset(): void
    {
        self::$queue = [];
        self::$requests = [];
    }

    public static function calls(): int
    {
        return \count(self::$requests);
    }

    public static function answer(int $status, string $body): void
    {
        self::$queue[] = new MockResponse($body, ['http_code' => $status]);
    }

    /** @param array<string, mixed> $volume what `items[0]` holds */
    public static function found(array $volume): void
    {
        self::answer(200, json_encode(['kind' => 'books#volumes', 'totalItems' => 1, 'items' => [$volume]], \JSON_THROW_ON_ERROR));
    }

    public static function notFound(): void
    {
        self::answer(200, json_encode(['kind' => 'books#volumes', 'totalItems' => 0], \JSON_THROW_ON_ERROR));
    }

    /**
     * A volume as Google sends it, with only what a test varies.
     *
     * @return array<string, mixed>
     */
    public static function volume(string $title = 'Inferno', string $id = 'abc123'): array
    {
        return [
            'id' => $id,
            'volumeInfo' => [
                'title' => $title,
                'subtitle' => 'A novel',
                'authors' => ['Dan Brown'],
                'publisher' => 'Planeta',
                'publishedDate' => '2013-05-14',
                'language' => 'es',
                'imageLinks' => ['thumbnail' => 'http://books.google.com/books/content?id='.$id.'&printsec=frontcover&img=1&zoom=1&edge=curl&source=gbs_api'],
            ],
        ];
    }

    public static function failWith(\Throwable $error): void
    {
        self::$queue[] = $error;
    }

    public static function client(): MockHttpClient
    {
        return new MockHttpClient(static function (string $method, string $url, array $options): MockResponse {
            $headers = [];
            $normalized = \is_array($options['normalized_headers'] ?? null) ? $options['normalized_headers'] : [];
            foreach ($normalized as $name => $lines) {
                $headers[(string) $name] = [];
                foreach (\is_array($lines) ? $lines : [] as $line) {
                    $headers[(string) $name][] = \is_string($line) ? $line : '';
                }
            }
            self::$requests[] = ['method' => $method, 'url' => $url, 'headers' => $headers];

            $next = array_shift(self::$queue);
            if ($next instanceof \Throwable) {
                throw $next;
            }

            // Nothing queued: a call nobody expected is an error the test can see (and count).
            return $next ?? new MockResponse('unexpected call', ['http_code' => 599]);
        });
    }
}
