<?php

declare(strict_types=1);

namespace App\Tests\Support;

use Symfony\Component\HttpClient\MockHttpClient;
use Symfony\Component\HttpClient\Response\MockResponse;

/**
 * Comic Vine, in memory: a queue of answers for the searches (in order) and a table of answers for the details (by
 * "volume/796" or "issue/6"), and every request remembered with its address, key included: Comic Vine only takes the
 * key there, and the tests check that it goes nowhere else. Static, because the test client boots a new kernel for
 * every request.
 */
final class FakeComicVineApi
{
    /** @var list<MockResponse|\Throwable> */
    private static array $searches = [];

    /** @var array<string, list<MockResponse|\Throwable>> */
    private static array $details = [];

    /** @var list<array{method: string, url: string, path: string, query: array<string, mixed>, headers: array<string, list<string>>}> */
    public static array $requests = [];

    public static function reset(): void
    {
        self::$searches = [];
        self::$details = [];
        self::$requests = [];
    }

    public static function calls(): int
    {
        return \count(self::$requests);
    }

    public static function searchCalls(): int
    {
        return \count(array_filter(self::$requests, static fn (array $r): bool => '/api/search/' === $r['path']));
    }

    public static function detailCalls(): int
    {
        return self::calls() - self::searchCalls();
    }

    /** @return list<string> the paths of the detail calls, in the order they were made */
    public static function detailPaths(): array
    {
        $paths = array_map(static fn (array $r): string => $r['path'], self::$requests);

        return array_values(array_filter($paths, static fn (string $p): bool => '/api/search/' !== $p));
    }

    /** @param list<mixed> $results */
    public static function searchFinds(array $results): void
    {
        self::searchAnswers(200, json_encode(['status_code' => 1, 'error' => 'OK', 'number_of_total_results' => \count($results), 'results' => $results], \JSON_THROW_ON_ERROR));
    }

    public static function searchAnswers(int $status, string $body): void
    {
        self::$searches[] = new MockResponse($body, ['http_code' => $status]);
    }

    public static function searchFails(\Throwable $error): void
    {
        self::$searches[] = $error;
    }

    /** The answer starts well and breaks while it is being read, the way a connection that drops does. */
    public static function searchBreaksWhileRead(\Throwable $error): void
    {
        self::$searches[] = new MockResponse(self::breaking($error), ['http_code' => 200]);
    }

    /** @param array<string, mixed> $credits `person_credits` and the like */
    public static function detailIs(string $resource, int $id, array $credits): void
    {
        self::detailAnswers($resource, $id, 200, json_encode(['status_code' => 1, 'error' => 'OK', 'results' => $credits], \JSON_THROW_ON_ERROR));
    }

    public static function detailAnswers(string $resource, int $id, int $status, string $body): void
    {
        self::$details[$resource.'/'.$id][] = new MockResponse($body, ['http_code' => $status]);
    }

    public static function detailFails(string $resource, int $id, \Throwable $error): void
    {
        self::$details[$resource.'/'.$id][] = $error;
    }

    public static function detailBreaksWhileRead(string $resource, int $id, \Throwable $error): void
    {
        self::$details[$resource.'/'.$id][] = new MockResponse(self::breaking($error), ['http_code' => 200]);
    }

    /** @return \Generator<int, string> */
    private static function breaking(\Throwable $error): \Generator
    {
        yield '{"results":[';

        throw $error;
    }

    /**
     * A series as the search sends it.
     *
     * @return array<string, mixed>
     */
    public static function volume(int $id = 796, string $name = 'Batman'): array
    {
        return [
            'id' => $id,
            'name' => $name,
            'resource_type' => 'volume',
            'start_year' => '1940',
            'publisher' => ['id' => 10, 'name' => 'DC Comics'],
            'image' => [
                'small_url' => 'https://comicvine.gamespot.com/a/uploads/scale_small/6/66037/1334617-batman_1_cover.jpg',
                'medium_url' => 'https://comicvine.gamespot.com/a/uploads/scale_medium/6/66037/1334617-batman_1_cover.jpg',
                'original_url' => 'https://comicvine.gamespot.com/a/uploads/original/6/66037/1334617-batman_1_cover.jpg',
            ],
        ];
    }

    /**
     * An issue as the search sends it.
     *
     * @return array<string, mixed>
     */
    public static function issue(int $id = 6, string $name = 'The Joker', string $number = '1'): array
    {
        return [
            'id' => $id,
            'name' => $name,
            'resource_type' => 'issue',
            'issue_number' => $number,
            'cover_date' => '1940-04-01',
            'store_date' => '1940-03-15',
            'volume' => ['id' => 796, 'name' => 'Batman'],
            'image' => [
                'small_url' => 'https://comicvine.gamespot.com/a/uploads/scale_small/6/66037/1334617-batman_1_cover.jpg',
                'medium_url' => 'https://comicvine.gamespot.com/a/uploads/scale_medium/6/66037/1334617-batman_1_cover.jpg',
            ],
        ];
    }

    /**
     * The credits of a detail.
     *
     * @return array<string, mixed>
     */
    public static function credits(): array
    {
        return ['person_credits' => [
            ['id' => 1, 'name' => 'Bill Finger', 'role' => 'writer'],
            ['id' => 2, 'name' => 'Bob Kane', 'role' => 'penciler, inker'],
            ['id' => 3, 'name' => 'Jerry Robinson', 'role' => 'colorist'],
        ]];
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
            $parts = parse_url($url);
            $path = \is_array($parts) ? ($parts['path'] ?? '') : '';
            parse_str(\is_array($parts) ? ($parts['query'] ?? '') : '', $parsed);
            $query = [];
            foreach ($parsed as $name => $value) {
                $query[(string) $name] = $value;
            }
            self::$requests[] = ['method' => $method, 'url' => $url, 'path' => $path, 'query' => $query, 'headers' => $headers];

            if ('/api/search/' === $path) {
                $next = array_shift(self::$searches);
            } elseif (1 === preg_match('#^/api/(volume/4050|issue/4000)-(\d+)/$#', $path, $match)) {
                $key = ('volume/4050' === $match[1] ? 'volume' : 'issue').'/'.$match[2];
                $queue = self::$details[$key] ?? [];
                $next = array_shift($queue);
                self::$details[$key] = $queue;
            } else {
                $next = new MockResponse('unexpected call', ['http_code' => 599]);
            }
            if ($next instanceof \Throwable) {
                throw $next;
            }

            // Nothing queued: a call nobody expected is an error the test can see (and count).
            return $next ?? new MockResponse('unexpected call', ['http_code' => 599]);
        });
    }
}
