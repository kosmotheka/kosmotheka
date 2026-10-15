<?php

declare(strict_types=1);

namespace App\Tests\Support;

use Symfony\Component\HttpClient\MockHttpClient;
use Symfony\Component\HttpClient\Response\MockResponse;

/**
 * The TMDB API, in memory: a queue of answers for the searches (in order) and a table of answers for the details (by
 * id), and every request remembered. Static, because the test client boots a new kernel for every request.
 */
final class FakeTmdbApi
{
    /** @var list<MockResponse|\Throwable> */
    private static array $searches = [];

    /** @var array<string, list<MockResponse|\Throwable>> the details by "movie/603" or "tv/1396" */
    private static array $details = [];

    /** @var list<array{method: string, url: string, path: string, query: array<string, mixed>, headers: array<string, list<string>>}> */
    public static array $requests = [];

    /** Runs while TMDB is answering a search: what another request does in the meantime. */
    public static ?\Closure $whileSearching = null;

    public static function reset(): void
    {
        self::$searches = [];
        self::$details = [];
        self::$requests = [];
        self::$whileSearching = null;
    }

    public static function calls(): int
    {
        return \count(self::$requests);
    }

    public static function searchCalls(): int
    {
        return \count(array_filter(self::$requests, static fn (array $r): bool => str_contains($r['path'], '/search/')));
    }

    public static function detailCalls(): int
    {
        return self::calls() - self::searchCalls();
    }

    /** @param list<mixed> $results */
    public static function searchFinds(array $results): void
    {
        self::searchAnswers(200, json_encode(['page' => 1, 'results' => $results, 'total_results' => \count($results)], \JSON_THROW_ON_ERROR));
    }

    public static function searchAnswers(int $status, string $body): void
    {
        self::$searches[] = new MockResponse($body, ['http_code' => $status]);
    }

    public static function searchFails(\Throwable $error): void
    {
        self::$searches[] = $error;
    }

    /** @param array<string, mixed> $detail */
    public static function detailIs(string $kind, int $id, array $detail): void
    {
        self::detailAnswers($kind, $id, 200, json_encode($detail, \JSON_THROW_ON_ERROR));
    }

    public static function detailAnswers(string $kind, int $id, int $status, string $body): void
    {
        self::$details[$kind.'/'.$id][] = new MockResponse($body, ['http_code' => $status]);
    }

    public static function detailFails(string $kind, int $id, \Throwable $error): void
    {
        self::$details[$kind.'/'.$id][] = $error;
    }

    /**
     * A film as the search sends it.
     *
     * @return array<string, mixed>
     */
    public static function film(int $id = 603, string $title = 'Matrix'): array
    {
        return ['id' => $id, 'title' => $title, 'original_title' => 'The Matrix', 'release_date' => '1999-03-30', 'original_language' => 'en', 'poster_path' => '/f89U3ADr1oiB1s9GkdPOEpXUk5H.jpg'];
    }

    /**
     * A series as the search sends it.
     *
     * @return array<string, mixed>
     */
    public static function show(int $id = 1396, string $name = 'Breaking Bad'): array
    {
        return ['id' => $id, 'name' => $name, 'original_name' => 'Breaking Bad', 'first_air_date' => '2008-01-20', 'original_language' => 'en', 'poster_path' => '/ggFHVNu6YYI5L9pCfOacjizRGt.jpg'];
    }

    /**
     * The detail of a film.
     *
     * @return array<string, mixed>
     */
    public static function filmDetail(): array
    {
        return ['runtime' => 136, 'credits' => ['crew' => [['job' => 'Director', 'name' => 'Lana Wachowski'], ['job' => 'Producer', 'name' => 'Joel Silver'], ['job' => 'Director', 'name' => 'Lilly Wachowski']]]];
    }

    /**
     * The detail of a series.
     *
     * @return array<string, mixed>
     */
    public static function showDetail(): array
    {
        return ['number_of_seasons' => 5, 'created_by' => [['name' => 'Vince Gilligan']]];
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

            if (1 === preg_match('#^/3/search/(movie|tv)$#', $path)) {
                $next = array_shift(self::$searches);
                if (self::$whileSearching instanceof \Closure) {
                    (self::$whileSearching)();
                }
            } elseif (1 === preg_match('#^/3/(movie|tv)/(\d+)$#', $path, $match)) {
                $queue = self::$details[$match[1].'/'.$match[2]] ?? [];
                $next = array_shift($queue);
                self::$details[$match[1].'/'.$match[2]] = $queue;
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
