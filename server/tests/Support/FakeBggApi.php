<?php

declare(strict_types=1);

namespace App\Tests\Support;

use Symfony\Component\HttpClient\MockHttpClient;
use Symfony\Component\HttpClient\Response\MockResponse;

/**
 * The XML API 2 of BoardGameGeek and RPGGeek, in memory: a queue of answers for the searches and another for the
 * details (in order), and every request remembered. The XML is written by hand after the documentation
 * (boardgamegeek.com/wiki/page/BGG_XML_API2); nothing here was recorded from the real service. Static, because the
 * test client boots a new kernel for every request.
 */
final class FakeBggApi
{
    /** @var list<MockResponse|\Throwable> */
    private static array $searches = [];

    /** @var list<MockResponse|\Throwable> */
    private static array $details = [];

    /** @var list<array{method: string, url: string, host: string, path: string, query: array<string, mixed>, headers: array<string, list<string>>}> */
    public static array $requests = [];

    /** Runs while the site is answering a search: what another request does in the meantime. */
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
        return \count(array_filter(self::$requests, static fn (array $r): bool => str_ends_with($r['path'], '/search')));
    }

    public static function detailCalls(): int
    {
        return self::calls() - self::searchCalls();
    }

    public static function searchAnswers(int $status, string $body): void
    {
        self::$searches[] = new MockResponse($body, ['http_code' => $status]);
    }

    public static function searchFails(\Throwable $error): void
    {
        self::$searches[] = $error;
    }

    public static function detailAnswers(int $status, string $body): void
    {
        self::$details[] = new MockResponse($body, ['http_code' => $status]);
    }

    public static function detailFails(\Throwable $error): void
    {
        self::$details[] = $error;
    }

    /** @param list<string> $items the `<item>` lines of {@see self::searchItem()} */
    public static function searchFinds(array $items): void
    {
        self::searchAnswers(200, self::items($items, \count($items)));
    }

    /** @param list<string> $items the `<item>` of {@see self::game()} or {@see self::book()} */
    public static function detailsAre(array $items): void
    {
        self::detailAnswers(200, self::items($items));
    }

    /** @param list<string> $items */
    public static function items(array $items, ?int $total = null): string
    {
        return '<?xml version="1.0" encoding="utf-8"?>'."\n".'<items termsofuse="https://boardgamegeek.com/xmlapi/termsofuse"'.(null === $total ? '' : ' total="'.$total.'"').'>'.implode('', $items).'</items>';
    }

    public static function searchItem(int $id = 13, string $name = 'CATAN', string $type = 'boardgame', string|int|null $year = 1995): string
    {
        return '<item type="'.$type.'" id="'.$id.'"><name type="primary" value="'.self::escape($name).'"/>'.(null === $year ? '' : '<yearpublished value="'.$year.'"/>').'</item>';
    }

    /**
     * A board game as `thing` sends it, or an expansion when `$expands` names the game it expands.
     *
     * @param array{0: int, 1: string}|null $expands
     */
    public static function game(int $id = 13, string $name = 'CATAN', ?array $expands = null, string $image = 'https://cf.geekdo-images.com/W3Bsga_uLP9kO91gZ7H8yw__original/img/o18bjC1GpdDPSHpdYqmZ2eLuUVU=/0x0/filters:format(jpeg)/pic2419375.jpg'): string
    {
        return '<item type="'.(null === $expands ? 'boardgame' : 'boardgameexpansion').'" id="'.$id.'">'
            .'<thumbnail>https://cf.geekdo-images.com/thumb.jpg</thumbnail><image>'.self::escape($image).'</image>'
            .'<name type="primary" sortindex="1" value="'.self::escape($name).'"/><name type="alternate" sortindex="1" value="Die Siedler von Catan"/>'
            .'<description>Players collect resources &amp;amp; build.&#10;&#10;Text from Wikipedia.</description>'
            .'<yearpublished value="1995"/><minplayers value="3"/><maxplayers value="4"/><playingtime value="120"/><minplaytime value="60"/><maxplaytime value="120"/><minage value="10"/>'
            .'<link type="boardgamecategory" id="1021" value="Economic"/>'
            .'<link type="boardgamedesigner" id="11" value="Klaus Teuber"/>'
            .'<link type="boardgameartist" id="2" value="Volkan Baga"/><link type="boardgameartist" id="3" value="Michael Menzel"/>'
            .'<link type="boardgamepublisher" id="37" value="KOSMOS"/><link type="boardgamepublisher" id="38" value="Devir"/>'
            .(null === $expands ? '<link type="boardgameexpansion" id="926" value="Catan: Seafarers"/>' : '<link type="boardgameexpansion" id="'.$expands[0].'" value="'.self::escape($expands[1]).'" inbound="true"/>')
            .'</item>';
    }

    /** A role-playing book as `thing` sends it on RPGGeek. */
    public static function book(int $id = 4000, string $name = 'Dungeon Master’s Guide'): string
    {
        return '<item type="rpgitem" id="'.$id.'"><image>https://cf.geekdo-images.com/rpg__original/img/abc=/0x0/pic1.png</image>'
            .'<name type="primary" value="'.self::escape($name).'"/><yearpublished value="1979"/>'
            .'<link type="rpgdesigner" id="1" value="Gary Gygax"/><link type="rpgartist" id="2" value="David Sutherland"/><link type="rpgpublisher" id="3" value="TSR"/></item>';
    }

    public static function escape(string $text): string
    {
        return htmlspecialchars($text, \ENT_QUOTES | \ENT_XML1);
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
            $host = \is_array($parts) ? ($parts['host'] ?? '') : '';
            parse_str(\is_array($parts) ? ($parts['query'] ?? '') : '', $parsed);
            $query = [];
            foreach ($parsed as $name => $value) {
                $query[(string) $name] = $value;
            }
            self::$requests[] = ['method' => $method, 'url' => $url, 'host' => $host, 'path' => $path, 'query' => $query, 'headers' => $headers];

            if ('/xmlapi2/search' === $path) {
                $next = array_shift(self::$searches);
                if (self::$whileSearching instanceof \Closure) {
                    (self::$whileSearching)();
                }
            } elseif ('/xmlapi2/thing' === $path) {
                $next = array_shift(self::$details);
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
