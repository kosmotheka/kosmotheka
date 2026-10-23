<?php

declare(strict_types=1);

namespace App\Tests\Support;

use Symfony\Component\HttpClient\MockHttpClient;
use Symfony\Component\HttpClient\Response\MockResponse;

/**
 * The GameUPC API, in memory: a queue of answers for the looks and another for the choices (in order), and every
 * request remembered with its headers and body. The JSON follows the examples of gameupc.com/gameupc-oas.yaml; the
 * verified game is the one the app recorded from the test server. Static, because the test client boots a new kernel
 * for every request.
 */
final class FakeGameUpcApi
{
    public const string GLOOMHAVEN_COVER = 'https://cf.geekdo-images.com/sZYp_3BTDGjh2unaZfZmuA__original/img/7d-lj5Gd1e8PFnD97LYFah2c45M=/0x0/filters:format(jpeg)/pic2437871.jpg';

    /** @var list<MockResponse|\Throwable> */
    private static array $looks = [];

    /** @var list<MockResponse|\Throwable> */
    private static array $choices = [];

    /** @var list<array{method: string, url: string, path: string, query: string, headers: array<string, list<string>>, body: string}> */
    public static array $requests = [];

    public static function reset(): void
    {
        self::$looks = [];
        self::$choices = [];
        self::$requests = [];
    }

    public static function calls(): int
    {
        return \count(self::$requests);
    }

    public static function choiceCalls(): int
    {
        return \count(array_filter(self::$requests, static fn (array $r): bool => 'POST' === $r['method']));
    }

    /** @param array<string, mixed> $body */
    public static function lookAnswers(array $body, int $status = 200): void
    {
        self::lookAnswersText(json_encode($body, \JSON_THROW_ON_ERROR), $status);
    }

    public static function lookAnswersText(string $body, int $status = 200): void
    {
        self::$looks[] = new MockResponse($body, ['http_code' => $status]);
    }

    public static function lookFails(\Throwable $error): void
    {
        self::$looks[] = $error;
    }

    public static function choiceAnswers(int $status = 200): void
    {
        self::$choices[] = new MockResponse('{"status":"ok"}', ['http_code' => $status]);
    }

    /**
     * A barcode GameUPC is sure of.
     *
     * @return array<string, mixed>
     */
    public static function verified(string $upc = '019962194719'): array
    {
        return [
            'status' => 'ok',
            'upc' => $upc,
            'name' => 'Gloomhaven',
            'searched_for' => 'Gloomhaven',
            'bgg_info_status' => 'verified',
            'bgg_info' => [self::game(174430, 'Gloomhaven', self::GLOOMHAVEN_COVER, 96)],
        ];
    }

    /**
     * A barcode GameUPC has guesses for.
     *
     * @param list<array<string, mixed>>|null $games
     *
     * @return array<string, mixed>
     */
    public static function guesses(?array $games = null, string $upc = '222222222224'): array
    {
        return [
            'status' => 'ok',
            'upc' => $upc,
            'name' => 'Tiny Towns',
            'searched_for' => 'Tiny Towns',
            'bgg_info_status' => 'choose_from_bgg_info_or_search',
            'bgg_info' => $games ?? [
                self::game(265736, 'Tiny Towns', null, 59),
                self::game(287576, 'Tiny Towns: Fortune', null, 0),
            ],
        ];
    }

    /**
     * A barcode GameUPC knows nothing of.
     *
     * @return array<string, mixed>
     */
    public static function nothing(string $upc = '333333333331'): array
    {
        return ['status' => 'ok', 'upc' => $upc, 'name' => '', 'searched_for' => '', 'bgg_info_status' => 'choose_from_bgg_info_or_search', 'bgg_info' => []];
    }

    /** @return array<string, mixed> */
    public static function game(mixed $id, mixed $name, mixed $image = null, int $confidence = 0): array
    {
        return [
            'id' => $id,
            'name' => $name,
            'thumbnail_url' => null,
            'image_url' => $image,
            'page_url' => 'https://boardgamegeek.com/boardgame/'.(\is_int($id) ? $id : 0),
            'update_url' => 'https://api.gameupc.com/v1/upc/x/bgg_id/'.(\is_int($id) ? $id : 0),
            'confidence' => $confidence,
        ];
    }

    public static function client(): MockHttpClient
    {
        return new MockHttpClient(static function (string $method, string $url, array $options): MockResponse {
            $parts = parse_url($url);
            $headers = [];
            foreach (\is_array($options['normalized_headers'] ?? null) ? $options['normalized_headers'] : [] as $name => $values) {
                $headers[(string) $name] = array_map(static fn (mixed $v): string => \is_string($v) ? $v : '', \is_array($values) ? array_values($values) : []);
            }
            $body = $options['body'] ?? '';
            self::$requests[] = [
                'method' => $method,
                'url' => $url,
                'path' => \is_array($parts) ? ($parts['path'] ?? '') : '',
                'query' => \is_array($parts) ? ($parts['query'] ?? '') : '',
                'headers' => $headers,
                'body' => \is_string($body) ? $body : '',
            ];

            $next = 'POST' === $method ? array_shift(self::$choices) : array_shift(self::$looks);
            if ($next instanceof \Throwable) {
                throw $next;
            }

            // Nothing queued: a call nobody expected is an error the test can see (and count).
            return $next ?? new MockResponse('unexpected call', ['http_code' => 599]);
        });
    }
}
