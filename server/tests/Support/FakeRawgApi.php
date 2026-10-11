<?php

declare(strict_types=1);

namespace App\Tests\Support;

use Symfony\Component\HttpClient\MockHttpClient;
use Symfony\Component\HttpClient\Response\MockResponse;

/**
 * The RAWG API, in memory: a queue of answers for the searches (in order) and a table of answers for the details (by
 * id), and every request remembered. RAWG takes its key only in the address, so the requests are kept with it: the
 * tests then check that it goes nowhere else. Static, because the test client boots a new kernel for every request.
 */
final class FakeRawgApi
{
    /** @var list<MockResponse|\Throwable> */
    private static array $searches = [];

    /** @var array<int, list<MockResponse|\Throwable>> */
    private static array $details = [];

    /** @var list<array{method: string, url: string, path: string, query: array<string, mixed>}> */
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
        return \count(array_filter(self::$requests, static fn (array $r): bool => '/api/games' === $r['path']));
    }

    public static function detailCalls(): int
    {
        return self::calls() - self::searchCalls();
    }

    /** @param list<mixed> $results */
    public static function searchFinds(array $results): void
    {
        self::searchAnswers(200, json_encode(['count' => \count($results), 'results' => $results], \JSON_THROW_ON_ERROR));
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

    public static function detailBreaksWhileRead(int $id, \Throwable $error): void
    {
        self::$details[$id][] = new MockResponse(self::breaking($error), ['http_code' => 200]);
    }

    /** @return \Generator<int, string> */
    private static function breaking(\Throwable $error): \Generator
    {
        yield '{"results":[';

        throw $error;
    }

    /** @param array<string, mixed> $detail */
    public static function detailIs(int $id, array $detail): void
    {
        self::detailAnswers($id, 200, json_encode($detail, \JSON_THROW_ON_ERROR));
    }

    public static function detailAnswers(int $id, int $status, string $body): void
    {
        self::$details[$id][] = new MockResponse($body, ['http_code' => $status]);
    }

    public static function detailFails(int $id, \Throwable $error): void
    {
        self::$details[$id][] = $error;
    }

    /**
     * A game as the search sends it.
     *
     * @return array<string, mixed>
     */
    public static function game(int $id = 3498, string $name = 'Grand Theft Auto V'): array
    {
        return [
            'id' => $id,
            'name' => $name,
            'released' => '2013-09-17',
            'background_image' => 'https://media.rawg.io/media/games/456/456dea5e1c7e3cd07060c14e96612001.jpg',
            'platforms' => [['platform' => ['id' => 4, 'name' => 'PC']], ['platform' => ['id' => 187, 'name' => 'PlayStation 5']]],
        ];
    }

    /**
     * The detail of a game.
     *
     * @return array<string, mixed>
     */
    public static function gameDetail(): array
    {
        return ['developers' => [['name' => 'Rockstar North']], 'publishers' => [['name' => 'Rockstar Games']]];
    }

    public static function client(): MockHttpClient
    {
        return new MockHttpClient(static function (string $method, string $url, array $options): MockResponse {
            $parts = parse_url($url);
            $path = \is_array($parts) ? ($parts['path'] ?? '') : '';
            parse_str(\is_array($parts) ? ($parts['query'] ?? '') : '', $parsed);
            $query = [];
            foreach ($parsed as $name => $value) {
                $query[(string) $name] = $value;
            }
            self::$requests[] = ['method' => $method, 'url' => $url, 'path' => $path, 'query' => $query];

            if ('/api/games' === $path) {
                $next = array_shift(self::$searches);
            } elseif (1 === preg_match('#^/api/games/(\d+)$#', $path, $match)) {
                $queue = self::$details[(int) $match[1]] ?? [];
                $next = array_shift($queue);
                self::$details[(int) $match[1]] = $queue;
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
