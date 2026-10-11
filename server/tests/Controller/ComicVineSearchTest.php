<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Entity\ActivityEntry;
use App\Entity\MetadataCacheEntry;
use App\Entity\User;
use App\Enum\ComicVineResource;
use App\Metadata\ComicVineSearch;
use App\Security\RequestLimiter;
use App\Tests\Support\DeviceAuthTestTrait;
use App\Tests\Support\FakeComicVineApi;
use App\Tests\Support\RecordingLogger;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\HttpClient\Exception\TransportException;

/**
 * Searching comics by title in Comic Vine through the server (7.8, D62): series and issues, the people of the first
 * three results. As with RAWG, two things are delicate: the title a person looks for (never in an address of ours,
 * never kept in the clear, never in a log) and the key, which Comic Vine only takes in the address of the outgoing
 * call. Hostile cases first.
 */
final class ComicVineSearchTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string KEY = 'SENTINEL-comicvine-key-8b41';
    private const string SECRET_TITLE = 'zxqv private comic';

    protected function tearDown(): void
    {
        $_ENV['COMIC_VINE_API_KEY'] = $_SERVER['COMIC_VINE_API_KEY'] = '';
        FakeComicVineApi::reset();
        RecordingLogger::reset();
        parent::tearDown();
    }

    // --- The way -----------------------------------------------------------------------------------------------------

    public function testASearchGivesSeriesFirstThenIssuesWithThePeopleOfTheFirstThree(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([
            FakeComicVineApi::issue(6, 'The Joker', '1'),
            FakeComicVineApi::volume(796, 'Batman'),
            FakeComicVineApi::issue(7, 'Second', '2'),
            FakeComicVineApi::volume(797, 'Detective'),
            FakeComicVineApi::volume(798, 'Robin'),
        ]);
        FakeComicVineApi::detailIs('volume', 796, ['person_credits' => [['name' => 'Someone', 'role' => 'writer']]]);
        FakeComicVineApi::detailIs('volume', 797, FakeComicVineApi::credits());
        FakeComicVineApi::detailIs('volume', 798, FakeComicVineApi::credits());

        $body = $this->search($client, $ada, 'batman');

        self::assertResponseStatusCodeSame(200);
        self::assertSame(['candidates'], array_keys($body));
        $candidates = $this->candidates($body);
        // The series first, in the order Comic Vine gave them, then the issues in theirs.
        self::assertSame([796, 797, 798, 6, 7], array_column($candidates, 'external_id'));
        self::assertSame(['volume', 'volume', 'volume', 'issue', 'issue'], array_column($candidates, 'resource'));
        self::assertSame([
            'resource' => 'volume',
            'external_id' => 796,
            'title' => 'Batman',
            'series' => null,
            'number' => null,
            'year' => 1940,
            'publisher' => 'DC Comics',
            'cover_url' => 'https://comicvine.gamespot.com/a/uploads/scale_medium/6/66037/1334617-batman_1_cover.jpg',
            'writers' => ['Someone'],
            'artists' => [],
        ], $candidates[0]);
        self::assertSame(['Bill Finger'], $candidates[1]['writers']);
        self::assertSame(['Bob Kane'], $candidates[1]['artists']);
        self::assertSame([
            'resource' => 'issue',
            'external_id' => 6,
            'title' => 'The Joker',
            'series' => 'Batman',
            'number' => '1',
            'year' => 1940,
            'publisher' => null,
            'cover_url' => 'https://comicvine.gamespot.com/a/uploads/scale_medium/6/66037/1334617-batman_1_cover.jpg',
            'writers' => [],
            'artists' => [],
        ], $candidates[3]);
        // One search, and the details of the first three only, by the number Comic Vine gave.
        self::assertSame(1, FakeComicVineApi::searchCalls());
        self::assertSame(['/api/volume/4050-796/', '/api/volume/4050-797/', '/api/volume/4050-798/'], FakeComicVineApi::detailPaths());
        $search = FakeComicVineApi::$requests[0];
        self::assertSame('GET', $search['method']);
        self::assertSame('batman', $search['query']['query'] ?? null);
        self::assertSame('volume,issue', $search['query']['resources'] ?? null);
        self::assertSame('json', $search['query']['format'] ?? null);
        self::assertSame('8', $search['query']['limit'] ?? null);
        self::assertSame(self::KEY, $search['query']['api_key'] ?? null);
        self::assertStringStartsWith('https://comicvine.gamespot.com/api/search/?', $search['url']);
        foreach (FakeComicVineApi::$requests as $request) {
            self::assertStringContainsString('Kosmotheka', $request['headers']['user-agent'][0] ?? '');
        }
    }

    public function testAnIssueDetailIsAskedByItsOwnAddress(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([FakeComicVineApi::issue(6)]);
        FakeComicVineApi::detailIs('issue', 6, FakeComicVineApi::credits());

        $candidate = $this->candidates($this->search($client, $ada, 'batman'))[0];

        self::assertSame(['/api/issue/4000-6/'], FakeComicVineApi::detailPaths());
        self::assertSame(['Bill Finger'], $candidate['writers']);
        self::assertSame(['Bob Kane'], $candidate['artists']);
    }

    public function testResultsOfAResourceThatWasNotAskedForAreDropped(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([
            ['id' => 1, 'name' => 'Batman', 'resource_type' => 'character'],
            ['id' => 2, 'name' => 'DC Comics', 'resource_type' => 'publisher'],
            ['id' => 3, 'name' => 'Bob Kane', 'resource_type' => 'person'],
            ['id' => 4, 'name' => 'Justice League', 'resource_type' => 'team'],
            ['id' => 5, 'name' => 'A video', 'resource_type' => 'video'],
            ['id' => 6, 'name' => 'No type'],
            ['id' => 7, 'name' => 'Odd type', 'resource_type' => ['volume']],
            ['id' => 8, 'name' => 'Another of our own words', 'resource_type' => 'search'],
            FakeComicVineApi::volume(796),
        ]);
        FakeComicVineApi::detailIs('volume', 796, FakeComicVineApi::credits());

        $body = $this->search($client, $ada, 'batman');

        self::assertSame([796], array_column($this->candidates($body), 'external_id'));
        self::assertSame(['/api/volume/4050-796/'], FakeComicVineApi::detailPaths());
    }

    public function testNothingFoundIsAnAnswerAndIsRemembered(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([]);

        $first = $this->search($client, $ada, 'qwertyuiop');
        $second = $this->search($client, $ada, 'qwertyuiop');

        self::assertResponseStatusCodeSame(200);
        self::assertSame(['candidates' => []], $first);
        self::assertSame($first, $second);
        self::assertSame(1, FakeComicVineApi::calls());
    }

    public function testTheServerListsComicVineOnlyWithAKey(): void
    {
        $client = $this->world();
        $client->request('GET', '/v1/server');
        self::assertContains('comic_vine', $this->offered($client));

        $_ENV['COMIC_VINE_API_KEY'] = $_SERVER['COMIC_VINE_API_KEY'] = '';
        self::ensureKernelShutdown();
        $client = self::createClient();
        $client->request('GET', '/v1/server');
        self::assertNotContains('comic_vine', $this->offered($client));
    }

    public function testWithoutAKeyTheRouteIsUnavailableAndNobodyIsAsked(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $_ENV['COMIC_VINE_API_KEY'] = $_SERVER['COMIC_VINE_API_KEY'] = '';
        self::ensureKernelShutdown();
        $client = self::createClient();

        $body = $this->search($client, $ada, 'batman');

        self::assertResponseStatusCodeSame(404);
        self::assertSame('metadata_unavailable', $body['error'] ?? null);
        self::assertSame(0, FakeComicVineApi::calls());
    }

    // --- 1. Who may ask -----------------------------------------------------------------------------------------------

    public function testWithoutASessionThereIsNoAnswerAndNoCall(): void
    {
        $client = $this->world();

        $client->request('POST', '/v1/metadata/comic-vine/search', server: ['CONTENT_TYPE' => 'application/json'], content: $this->payload('batman'));

        self::assertResponseStatusCodeSame(401);
        self::assertSame(0, FakeComicVineApi::calls());
    }

    public function testAnInvalidTokenGetsNoAnswerAndNoCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        $this->search($client, ['headers' => $this->deviceHeaders($ada['device_id'], 'nope')], 'batman');

        self::assertResponseStatusCodeSame(401);
        self::assertSame(0, FakeComicVineApi::calls());
    }

    public function testAnAccountNoLongerConfirmedGetsNoAnswerAndNoCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $this->entityManager()->createQuery(\sprintf('UPDATE %s u SET u.confirmedAt = NULL', User::class))->execute();

        $body = $this->search($client, $ada, 'batman');

        self::assertResponseStatusCodeSame(403);
        self::assertSame('account_not_confirmed', $body['error'] ?? null);
        self::assertSame(0, FakeComicVineApi::calls());
    }

    public function testABlockedAccountGetsNoAnswerAndNoCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $this->user('ada@example.com')->block(new \DateTimeImmutable());
        $this->entityManager()->flush();

        $body = $this->search($client, $ada, 'batman');

        self::assertResponseStatusCodeSame(403);
        self::assertSame('account_blocked', $body['error'] ?? null);
        self::assertSame(0, FakeComicVineApi::calls());
    }

    // --- 2. A body that is not the one asked -------------------------------------------------------------------------

    /** @return iterable<string, array{string}> */
    public static function badBodies(): iterable
    {
        yield 'an empty query' => ['{"query":""}'];
        yield 'a query of blanks' => ['{"query":"   "}'];
        yield 'a query of one letter' => ['{"query":"a"}'];
        yield 'no query' => ['{}'];
        yield 'a query that is not text' => ['{"query":{"a":1}}'];
        yield 'a line break' => ['{"query":"bat\nman"}'];
        yield 'a tab' => ['{"query":"bat\tman"}'];
        yield 'a null byte' => ['{"query":"bat\u0000man"}'];
        yield 'a control character' => ['{"query":"bat\u0007man"}'];
        yield 'more fields' => ['{"query":"batman","resources":"character"}'];
        yield 'a key' => ['{"query":"batman","api_key":"x"}'];
        yield 'broken json' => ['{"query":"batman"'];
        yield 'a list' => ['["batman"]'];
        yield 'a string' => ['"batman"'];
        yield 'nothing' => [''];
    }

    #[DataProvider('badBodies')]
    public function testABodyThatIsNotTheOneAskedIsRefusedWithoutComicVineAndWithoutARow(string $body): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        $client->request('POST', '/v1/metadata/comic-vine/search', server: $ada['headers'] + ['CONTENT_TYPE' => 'application/json'], content: $body);

        self::assertGreaterThanOrEqual(400, $client->getResponse()->getStatusCode());
        self::assertLessThan(500, $client->getResponse()->getStatusCode());
        self::assertSame(0, FakeComicVineApi::calls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testAQueryOfOneOverTheLimitAndOfAMegabyteAreRefused(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        foreach ([str_repeat('x', 101), str_repeat('x', 1_000_000)] as $query) {
            $this->search($client, $ada, $query);
            self::assertResponseStatusCodeSame(422);
        }

        self::assertSame(0, FakeComicVineApi::calls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testSurroundingBlanksAndRepeatedSpacesDoNotChangeWhatIsAsked(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([]);

        $this->search($client, $ada, "  Amazing   Spider \u{a0}");

        self::assertSame('amazing spider', FakeComicVineApi::$requests[0]['query']['query'] ?? null);
    }

    // --- 3. A title that tries to be more than a title ---------------------------------------------------------------

    public function testATitleWithSymbolsTravelsAsOneEncodedParameter(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        foreach (['a&api_key=other', 'what? #1', 'a/../../b', '100%00 sure', 'x&resources=character&limit=100', 'ñ ü 漫画'] as $title) {
            FakeComicVineApi::searchFinds([]);
            $this->search($client, $ada, $title);
            self::assertResponseStatusCodeSame(200, $title);
            $request = FakeComicVineApi::$requests[\count(FakeComicVineApi::$requests) - 1];
            self::assertSame('/api/search/', $request['path'], $title);
            self::assertSame(mb_strtolower($title), $request['query']['query'] ?? null, $title);
            self::assertEqualsCanonicalizing(['api_key', 'format', 'query', 'resources', 'limit', 'field_list'], array_keys($request['query']), $title);
            self::assertSame('volume,issue', $request['query']['resources']);
            self::assertSame('8', $request['query']['limit']);
            self::assertSame(self::KEY, $request['query']['api_key']);
            self::assertSame(1, substr_count($request['url'], '?'), $title);
            self::assertSame(0, substr_count($request['url'], '#'), $title);
        }
    }

    // --- 4 and 11. The key, which travels in the outgoing address, never shows ----------------------------------------

    public function testTheKeyIsNowhereInWhatTheServerAnswersStoresOrLogsWhateverGoesWrong(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $seen = [];
        $n = 0;
        $withKey = 'https://comicvine.gamespot.com/api/search/?api_key='.self::KEY.'&query=x';

        // Comic Vine echoing the address in its errors, every status that is not a success.
        foreach ([401, 403, 420, 429, 404, 400, 500, 502] as $status) {
            $times = $status >= 500 ? 3 : 1;
            for ($i = 0; $i < $times; ++$i) {
                FakeComicVineApi::searchAnswers($status, '{"error":"bad request '.$withKey.'","status_code":100,"results":[]}');
            }
            $this->clearBrake();
            $seen[] = $this->raw($client, $ada, 'query '.++$n);
        }
        // The same said with a success status: its own status code is the one that tells.
        FakeComicVineApi::searchAnswers(200, '{"error":"Invalid API Key '.$withKey.'","status_code":100,"results":[]}');
        $this->clearBrake();
        $seen[] = $this->raw($client, $ada, 'query '.++$n);
        // The http client failing the way it does: its messages carry the address.
        foreach ([
            new TransportException('Could not resolve host for "'.$withKey.'"'),
            new TransportException('Idle timeout reached for "'.$withKey.'"'),
            new \RuntimeException('Failed '.$withKey, previous: new TransportException($withKey)),
        ] as $error) {
            FakeComicVineApi::searchFails($error);
            $this->clearBrake();
            $seen[] = $this->raw($client, $ada, 'query '.++$n);
        }
        // A connection that drops while it is being read.
        FakeComicVineApi::searchBreaksWhileRead(new TransportException('Connection reset for "'.$withKey.'"'));
        $this->clearBrake();
        $seen[] = $this->raw($client, $ada, 'query '.++$n);
        // Rubbish that is not JSON, and JSON that echoes it.
        FakeComicVineApi::searchAnswers(200, '<html>'.$withKey.'</html>');
        $this->clearBrake();
        $seen[] = $this->raw($client, $ada, 'query '.++$n);
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume(7, 'Echo '.$withKey)]);
        FakeComicVineApi::detailAnswers('volume', 7, 500, $withKey);
        $this->clearBrake();
        $seen[] = $this->raw($client, $ada, 'query '.++$n);
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume(8)]);
        FakeComicVineApi::detailFails('volume', 8, new TransportException('timeout for "'.$withKey.'"'));
        $this->clearBrake();
        $seen[] = $this->raw($client, $ada, 'query '.++$n);
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume(9)]);
        FakeComicVineApi::detailBreaksWhileRead('volume', 9, new TransportException('reset for "'.$withKey.'"'));
        $this->clearBrake();
        $seen[] = $this->raw($client, $ada, 'query '.++$n);

        foreach ($seen as $answer) {
            self::assertStringNotContainsString(self::KEY, $answer);
        }
        foreach ($this->allCacheRowsAsText() as $row) {
            self::assertStringNotContainsString(self::KEY, $row);
        }
        self::assertNotSame('', RecordingLogger::everything(), 'the logger must be the one the test reads');
        self::assertStringNotContainsString(self::KEY, RecordingLogger::everything());
        self::assertStringNotContainsString('api_key=', RecordingLogger::everything());
    }

    // --- 5, 12, 13, 19 and 20. A Comic Vine that sends rubbish --------------------------------------------------------

    public function testAThousandResultsReachTheAppAsEightAndOnlyThreeDetailsAreAsked(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $results = [];
        for ($i = 1; $i <= 1000; ++$i) {
            $results[] = FakeComicVineApi::volume($i, 'V'.$i);
        }
        FakeComicVineApi::searchFinds($results);

        $body = $this->search($client, $ada, 'comic');

        self::assertResponseStatusCodeSame(200);
        self::assertSame(range(1, 8), array_column($this->candidates($body), 'external_id'));
        self::assertSame(3, FakeComicVineApi::detailCalls());
    }

    public function testFieldsOfTheWrongTypeAndHugeOnesAreCutOrDropped(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([
            ['id' => 7, 'name' => str_repeat('T', 30_000), 'resource_type' => 'volume', 'start_year' => 'long ago', 'publisher' => 'DC', 'image' => 'x', 'deck' => str_repeat('o', 1000)],
            ['id' => 8, 'name' => ['x'], 'resource_type' => 'volume', 'publisher' => ['name' => str_repeat('P', 5000)], 'start_year' => ['1940']],
            ['id' => 9, 'resource_type' => 'issue', 'name' => 'Odd', 'issue_number' => ['1'], 'volume' => 'Batman', 'cover_date' => 5],
            ['id' => 10, 'resource_type' => 'issue', 'name' => 'Long', 'issue_number' => str_repeat('9', 5000), 'volume' => ['name' => str_repeat('S', 5000)]],
        ]);
        $people = [];
        for ($i = 0; $i < 100; ++$i) {
            $people[] = ['name' => 'Writer '.$i, 'role' => 'writer'];
        }
        $people[] = ['name' => str_repeat('N', 5000), 'role' => 'writer'];
        $people[] = 'x';
        $people[] = ['name' => ['y'], 'role' => 'writer'];
        $people[] = ['name' => 'Role array', 'role' => ['writer']];
        FakeComicVineApi::detailIs('volume', 7, ['person_credits' => $people]);
        FakeComicVineApi::detailIs('volume', 8, ['person_credits' => 'nobody']);

        $body = $this->search($client, $ada, 'comic');

        self::assertResponseStatusCodeSame(200);
        $candidates = $this->candidates($body);
        self::assertSame([7, 8, 9, 10], array_column($candidates, 'external_id'));
        $first = $candidates[0];
        self::assertIsString($first['title']);
        self::assertSame(300, mb_strlen($first['title']));
        self::assertNull($first['year']);
        self::assertNull($first['publisher']);
        self::assertNull($first['cover_url']);
        self::assertIsArray($first['writers']);
        self::assertCount(5, $first['writers']);
        self::assertSame('Writer 0', $first['writers'][0]);
        self::assertNull($candidates[1]['title']);
        self::assertNull($candidates[1]['year']);
        self::assertIsString($candidates[1]['publisher']);
        self::assertSame(200, mb_strlen($candidates[1]['publisher']));
        self::assertSame([], $candidates[1]['writers']);
        self::assertNull($candidates[2]['series']);
        self::assertNull($candidates[2]['number']);
        self::assertNull($candidates[2]['year']);
        self::assertIsString($candidates[3]['series']);
        self::assertSame(300, mb_strlen($candidates[3]['series']));
        self::assertIsString($candidates[3]['number']);
        self::assertLessThanOrEqual(20, mb_strlen($candidates[3]['number']));
    }

    /** @return iterable<string, array{mixed}> */
    public static function badIds(): iterable
    {
        yield 'a string' => ['abc'];
        yield 'a number as text' => ['796'];
        yield 'an array' => [[796]];
        yield 'negative' => [-4];
        yield 'zero' => [0];
        yield 'a float' => [1.5];
        yield 'too big' => [99_999_999_999_999];
        yield 'a path' => ['../../etc'];
        yield 'null' => [null];
    }

    #[DataProvider('badIds')]
    public function testAnIdThatIsNotAReasonablePositiveIntegerNeverReachesAnAddress(mixed $id): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([
            ['id' => $id, 'name' => 'Bad volume', 'resource_type' => 'volume'],
            ['id' => $id, 'name' => 'Bad issue', 'resource_type' => 'issue', 'volume' => ['name' => 'V']],
            FakeComicVineApi::volume(796),
        ]);
        FakeComicVineApi::detailIs('volume', 796, FakeComicVineApi::credits());

        $body = $this->search($client, $ada, 'comic');

        self::assertResponseStatusCodeSame(200);
        self::assertSame([796], array_column($this->candidates($body), 'external_id'));
        self::assertSame(['/api/volume/4050-796/'], FakeComicVineApi::detailPaths());
    }

    /** @return iterable<string, array{string}> */
    public static function badCovers(): iterable
    {
        yield 'a parent directory' => ['https://comicvine.gamespot.com/a/uploads/../x.jpg'];
        yield 'a parent directory encoded' => ['https://comicvine.gamespot.com/a/uploads/%2e%2e/x.jpg'];
        yield 'another site' => ['https://evil.example/a/uploads/scale_medium/x.jpg'];
        yield 'a look-alike host' => ['https://comicvine.gamespot.com.evil.example/a/uploads/x.jpg'];
        yield 'a user in the address' => ['https://comicvine.gamespot.com@evil.example/a/uploads/x.jpg'];
        yield 'a user before the host' => ['https://user@comicvine.gamespot.com/a/uploads/x.jpg'];
        yield 'a port' => ['https://comicvine.gamespot.com:8443/a/uploads/x.jpg'];
        yield 'http' => ['http://comicvine.gamespot.com/a/uploads/x.jpg'];
        yield 'a protocol-relative one' => ['//comicvine.gamespot.com/a/uploads/x.jpg'];
        yield 'another directory' => ['https://comicvine.gamespot.com/other/x.jpg'];
        yield 'not an image' => ['https://comicvine.gamespot.com/a/uploads/x.svg'];
        yield 'a script' => ['https://comicvine.gamespot.com/a/uploads/x.html'];
        yield 'no extension' => ['https://comicvine.gamespot.com/a/uploads/x'];
        yield 'a query' => ['https://comicvine.gamespot.com/a/uploads/x.jpg?evil=1'];
        yield 'a fragment' => ['https://comicvine.gamespot.com/a/uploads/x.jpg#evil'];
        yield 'another address inside' => ['https://comicvine.gamespot.com/a/uploads/https://evil.example/x.jpg'];
        yield 'a line break' => ["https://comicvine.gamespot.com/a/uploads/x.jpg\n"];
        yield 'a space' => ['https://comicvine.gamespot.com/a/uploads/x y.jpg'];
        yield 'an empty segment' => ['https://comicvine.gamespot.com/a/uploads//x.jpg'];
        yield 'the api itself' => ['https://comicvine.gamespot.com/api/search/?api_key=x.jpg'];
        yield 'very long' => ['https://comicvine.gamespot.com/a/uploads/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.jpg'];
    }

    #[DataProvider('badCovers')]
    public function testACoverThatIsNotAPlainImageOfTheirHostIsDropped(string $cover): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([
            ['id' => 5, 'name' => 'V', 'resource_type' => 'volume', 'image' => ['medium_url' => $cover, 'small_url' => $cover, 'original_url' => $cover]],
            ['id' => 6, 'name' => 'I', 'resource_type' => 'issue', 'volume' => ['name' => 'V'], 'image' => ['medium_url' => $cover]],
            ['id' => 7, 'name' => 'Array', 'resource_type' => 'volume', 'image' => ['medium_url' => [$cover]]],
        ]);

        $body = $this->search($client, $ada, 'comic');

        self::assertResponseStatusCodeSame(200);
        foreach ($this->candidates($body) as $candidate) {
            self::assertNull($candidate['cover_url']);
        }
    }

    /** @return iterable<string, array{string}> */
    public static function goodCovers(): iterable
    {
        yield 'jpg' => ['https://comicvine.gamespot.com/a/uploads/scale_medium/6/66037/1334617-batman_1_cover.jpg'];
        yield 'jpeg' => ['https://comicvine.gamespot.com/a/uploads/scale_small/0/40/a.jpeg'];
        yield 'png' => ['https://comicvine.gamespot.com/a/uploads/scale_medium/0/40/a_b-c.png'];
        yield 'webp' => ['https://comicvine.gamespot.com/a/uploads/scale_medium/0/40/a.webp'];
        yield 'gif' => ['https://comicvine.gamespot.com/a/uploads/scale_medium/0/40/a.gif'];
    }

    #[DataProvider('goodCovers')]
    public function testACoverOfTheirImageHostIsKept(string $cover): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([['id' => 5, 'name' => 'V', 'resource_type' => 'volume', 'image' => ['medium_url' => $cover]]]);

        $body = $this->search($client, $ada, 'comic');

        self::assertSame($cover, $this->candidates($body)[0]['cover_url']);
    }

    public function testTheOriginalImageIsNeverTheCoverBecauseItCanBeHuge(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([['id' => 5, 'name' => 'V', 'resource_type' => 'volume', 'image' => ['original_url' => 'https://comicvine.gamespot.com/a/uploads/original/0/40/a.jpg']]]);

        $body = $this->search($client, $ada, 'comic');

        self::assertNull($this->candidates($body)[0]['cover_url']);
    }

    /** @return iterable<string, array{string, list<string>, list<string>}> */
    public static function roles(): iterable
    {
        yield 'writer' => ['writer', ['W'], []];
        yield 'penciler' => ['penciler', [], ['W']];
        yield 'penciller' => ['penciller', [], ['W']];
        yield 'inker' => ['inker', [], ['W']];
        yield 'artist' => ['artist', [], ['W']];
        yield 'two roles' => ['writer, penciler', ['W'], ['W']];
        yield 'capitals and blanks' => ['  Writer ,  PENCILER ', ['W'], ['W']];
        yield 'colorist only' => ['colorist', [], []];
        yield 'letterer and editor' => ['letterer, editor, cover', [], []];
        yield 'no role' => ['', [], []];
        yield 'a role that only looks like one' => ['writers', [], []];
    }

    /**
     * @param list<string> $writers
     * @param list<string> $artists
     */
    #[DataProvider('roles')]
    public function testOnlyTheRolesThatFitAreKept(string $role, array $writers, array $artists): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([FakeComicVineApi::issue(6)]);
        FakeComicVineApi::detailIs('issue', 6, ['person_credits' => [['name' => 'W', 'role' => $role]]]);

        $candidate = $this->candidates($this->search($client, $ada, 'comic'))[0];

        self::assertSame($writers, $candidate['writers'], $role);
        self::assertSame($artists, $candidate['artists'], $role);
    }

    public function testACreditWithNoRoleAtAllIsOmitted(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume(796)]);
        FakeComicVineApi::detailIs('volume', 796, ['person_credits' => [['name' => 'Nameless role'], ['id' => 3]]]);

        $candidate = $this->candidates($this->search($client, $ada, 'comic'))[0];

        self::assertSame([], $candidate['writers']);
        self::assertSame([], $candidate['artists']);
    }

    public function testAnswersThatAreNotUsableAreAFailureAndLeaveNoRow(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $n = 0;

        foreach (['<html>hello</html>', '[1,2,3', 'null', '"text"', '{"results":"no"}', '{}', '{"status_code":1}', '{"status_code":104,"results":[]}', str_repeat('{"a":1,', 400_000)] as $nonsense) {
            FakeComicVineApi::searchAnswers(200, $nonsense);
            $body = $this->search($client, $ada, 'query '.++$n);
            self::assertResponseStatusCodeSame(502, substr($nonsense, 0, 20));
            self::assertSame('metadata_failed', $body['error'] ?? null);
        }

        self::assertSame(0, $this->cacheRows());
    }

    public function testAnEnormousAnswerIsAFailureAndLeavesNoRow(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume(1, str_repeat('T', 1_000_000))]);

        $body = $this->search($client, $ada, 'comic');

        self::assertResponseStatusCodeSame(502);
        self::assertSame('metadata_failed', $body['error'] ?? null);
        self::assertSame(0, $this->cacheRows());
    }

    // --- 6. Comic Vine says no ----------------------------------------------------------------------------------------

    /** @return iterable<string, array{int, string}> */
    public static function refusals(): iterable
    {
        yield 'too many requests' => [429, '{"detail":"no"}'];
        yield 'unauthorised' => [401, '{"status_code":100,"error":"Invalid API Key","results":[]}'];
        yield 'forbidden' => [403, '{}'];
        yield 'enhance your calm' => [420, '{}'];
        yield 'an invalid key said with a success status' => [200, '{"status_code":100,"error":"Invalid API Key","results":[]}'];
    }

    #[DataProvider('refusals')]
    public function testComicVineRefusingIsReportedAndNothingIsStored(int $status, string $body): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchAnswers($status, $body);

        $answer = $this->search($client, $ada, 'comic');

        self::assertResponseStatusCodeSame(503);
        self::assertSame('metadata_quota_exhausted', $answer['error'] ?? null);
        self::assertSame(0, $this->cacheRows());
    }

    public function testAfterARefusalTheServerStopsAskingComicVineForAWhile(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchAnswers(429, '{}');

        $this->search($client, $ada, 'comic');
        for ($try = 0; $try < 5; ++$try) {
            $body = $this->search($client, $ada, 'other '.$try);
            self::assertSame('metadata_quota_exhausted', $body['error'] ?? null);
        }

        self::assertSame(1, FakeComicVineApi::calls());
        self::assertGreaterThan(0, (int) $client->getResponse()->headers->get('Retry-After'));

        $this->clearBrake();
        FakeComicVineApi::searchFinds([]);
        $this->search($client, $ada, 'comic');
        self::assertResponseStatusCodeSame(200);
        self::assertSame(2, FakeComicVineApi::calls());
    }

    public function testAServerErrorIsTriedAgainAndThenAnswers(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchAnswers(503, 'down');
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume()]);
        FakeComicVineApi::detailIs('volume', 796, FakeComicVineApi::credits());

        $body = $this->search($client, $ada, 'comic');

        self::assertResponseStatusCodeSame(200);
        self::assertCount(1, $this->candidates($body));
        self::assertSame(2, FakeComicVineApi::searchCalls());
    }

    public function testAServerErrorThatKeepsComingStopsAtThreeTriesAndLeavesNoRow(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        for ($i = 0; $i < 5; ++$i) {
            FakeComicVineApi::searchAnswers(500, 'oops');
        }

        $body = $this->search($client, $ada, 'comic');

        self::assertResponseStatusCodeSame(502);
        self::assertSame('metadata_failed', $body['error'] ?? null);
        self::assertSame(3, FakeComicVineApi::searchCalls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testASlowComicVineIsNotAskedAgain(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFails(new TransportException('Idle timeout reached'));

        $this->search($client, $ada, 'comic');

        self::assertResponseStatusCodeSame(502);
        self::assertSame(1, FakeComicVineApi::calls());
    }

    // --- 7. A detail that fails ---------------------------------------------------------------------------------------

    public function testADetailThatFailsLeavesThatCandidateIncompleteAndTheSearchAnswers(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume(1, 'One'), FakeComicVineApi::volume(2, 'Two'), FakeComicVineApi::volume(3, 'Three')]);
        FakeComicVineApi::detailIs('volume', 1, FakeComicVineApi::credits());
        FakeComicVineApi::detailAnswers('volume', 2, 500, 'oops');
        FakeComicVineApi::detailFails('volume', 3, new TransportException('Idle timeout reached'));

        $body = $this->search($client, $ada, 'comic');

        self::assertResponseStatusCodeSame(200);
        $candidates = $this->candidates($body);
        self::assertCount(3, $candidates);
        self::assertSame(['Bill Finger'], $candidates[0]['writers']);
        foreach (\array_slice($candidates, 1) as $candidate) {
            self::assertSame([], $candidate['writers']);
            self::assertNotNull($candidate['title']);
        }
    }

    public function testADetailThatFailsDoesNotCostTheOthersTheirs(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume(1, 'One'), FakeComicVineApi::volume(2, 'Two'), FakeComicVineApi::volume(3, 'Three')]);
        FakeComicVineApi::detailAnswers('volume', 1, 500, 'oops');
        FakeComicVineApi::detailIs('volume', 2, FakeComicVineApi::credits());
        FakeComicVineApi::detailIs('volume', 3, FakeComicVineApi::credits());

        $candidates = $this->candidates($this->search($client, $ada, 'comic'));

        self::assertSame([], $candidates[0]['writers']);
        self::assertSame(['Bill Finger'], $candidates[1]['writers']);
        self::assertSame(['Bill Finger'], $candidates[2]['writers']);
    }

    public function testADetailRefusedOnQuotaPausesComicVineForTheNextSearch(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume()]);
        FakeComicVineApi::detailAnswers('volume', 796, 429, '{}');

        $this->search($client, $ada, 'comic');
        self::assertResponseStatusCodeSame(200);
        $calls = FakeComicVineApi::calls();
        $body = $this->search($client, $ada, 'something else');

        self::assertResponseStatusCodeSame(503);
        self::assertSame('metadata_quota_exhausted', $body['error'] ?? null);
        self::assertSame($calls, FakeComicVineApi::calls());
    }

    public function testADetailThatFailedIsNotRememberedAndIsAskedAgainNextTime(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume()]);
        FakeComicVineApi::detailAnswers('volume', 796, 500, 'oops');

        $this->search($client, $ada, 'comic');
        self::assertSame(0, $this->detailRows());
        FakeComicVineApi::detailIs('volume', 796, FakeComicVineApi::credits());
        $second = $this->search($client, $ada, 'comic');

        self::assertSame(['Bill Finger'], $this->candidates($second)[0]['writers']);
        self::assertSame(1, FakeComicVineApi::searchCalls());
        self::assertSame(1, $this->detailRows());
    }

    // --- 8. Nobody is remembered --------------------------------------------------------------------------------------

    public function testTwoAccountsLookingForTheSameCostOneRoundOfCallsAndNoRowSaysWhoOrWhat(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume()]);
        FakeComicVineApi::detailIs('volume', 796, FakeComicVineApi::credits());
        $activity = $this->entityManager()->getRepository(ActivityEntry::class)->count([]);

        $this->search($client, $ada, self::SECRET_TITLE);
        $this->search($client, $grace, '  '.mb_strtoupper(self::SECRET_TITLE).' ');

        self::assertSame(2, FakeComicVineApi::calls());
        self::assertSame(2, $this->cacheRows());
        self::assertSame(['id', 'provider', 'code', 'found', 'payload', 'fetched_at'], $this->cacheColumns());
        $rows = implode(' ', $this->allCacheRowsAsText());
        self::assertStringNotContainsString('ada@example.com', $rows);
        self::assertStringNotContainsString('grace@example.com', $rows);
        self::assertStringNotContainsString($this->user('ada@example.com')->getId()->toRfc4122(), $rows);
        self::assertStringNotContainsStringIgnoringCase('zxqv', $rows);
        self::assertStringNotContainsStringIgnoringCase('private comic', $rows);
        self::assertContains(ComicVineSearch::fingerprint(self::SECRET_TITLE), $this->codes());
        self::assertSame($activity, $this->entityManager()->getRepository(ActivityEntry::class)->count([]));
    }

    public function testTheSearchedTitleReachesNoLogEvenWhenSomethingGoesWrong(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchAnswers(429, '{}');
        $this->search($client, $ada, self::SECRET_TITLE);
        $this->clearBrake();
        FakeComicVineApi::searchAnswers(500, 'x');
        FakeComicVineApi::searchAnswers(500, 'x');
        FakeComicVineApi::searchAnswers(500, 'x');
        $this->search($client, $ada, self::SECRET_TITLE.' two');
        $this->search($client, $ada, self::SECRET_TITLE."\n");
        $client->request('POST', '/v1/metadata/comic-vine/search', server: $ada['headers'] + ['CONTENT_TYPE' => 'application/json'], content: '{"query":"'.self::SECRET_TITLE.'"');
        $client->request('POST', '/v1/metadata/comic-vine/search', server: $ada['headers'] + ['CONTENT_TYPE' => 'application/json'], content: '{"query":"'.self::SECRET_TITLE.'","extra":1}');

        self::assertNotSame('', RecordingLogger::everything(), 'the logger must be the one the test reads');
        self::assertStringNotContainsStringIgnoringCase('zxqv', RecordingLogger::everything());
        self::assertStringNotContainsStringIgnoringCase('private comic', RecordingLogger::everything());
    }

    // --- 9 and 21. Limits ---------------------------------------------------------------------------------------------

    public function testAnAccountPastItsSharedLimitIsCutAndTheOthersAreNot(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        $limiter = self::getContainer()->get(RequestLimiter::class);
        for ($i = 0; $i < 599; ++$i) {
            $limiter->consumeMetadata($this->user('ada@example.com')->getId()->toRfc4122());
        }
        FakeComicVineApi::searchFinds([]);
        FakeComicVineApi::searchFinds([]);

        $this->search($client, $ada, 'comic');
        self::assertResponseStatusCodeSame(200);
        $body = $this->search($client, $ada, 'comic');
        self::assertResponseStatusCodeSame(429);
        self::assertSame('too_many_requests', $body['error'] ?? null);

        $this->search($client, $grace, 'comic');
        self::assertResponseStatusCodeSame(200);
    }

    public function testWhenTheSearchesOfTheHourAreSpentTheCacheKeepsServing(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume()]);
        FakeComicVineApi::detailIs('volume', 796, FakeComicVineApi::credits());
        $this->search($client, $ada, 'comic');
        $limiter = self::getContainer()->get(RequestLimiter::class);
        for ($i = 0; $i < 149; ++$i) {
            $limiter->consumeComicVineCall(ComicVineResource::Search); // with the one made above, the 150 of the hour
        }

        $cached = $this->search($client, $ada, 'comic');
        self::assertResponseStatusCodeSame(200);
        self::assertSame('Batman', $this->candidates($cached)[0]['title']);
        self::assertSame(['Bill Finger'], $this->candidates($cached)[0]['writers']);

        $other = $this->search($client, $ada, 'other');
        self::assertResponseStatusCodeSame(503);
        self::assertSame('metadata_quota_exhausted', $other['error'] ?? null);
        self::assertSame(2, FakeComicVineApi::calls());
    }

    public function testTheLimitOfOneResourceDoesNotTakeTheOthersDown(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $limiter = self::getContainer()->get(RequestLimiter::class);
        for ($i = 0; $i < 150; ++$i) {
            $limiter->consumeComicVineCall(ComicVineResource::Issue); // the details of issues are spent
        }
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume(796), FakeComicVineApi::issue(6), FakeComicVineApi::issue(7)]);
        FakeComicVineApi::detailIs('volume', 796, FakeComicVineApi::credits());
        FakeComicVineApi::detailIs('issue', 6, FakeComicVineApi::credits());
        FakeComicVineApi::detailIs('issue', 7, FakeComicVineApi::credits());

        $body = $this->search($client, $ada, 'comic');

        self::assertResponseStatusCodeSame(200);
        $candidates = $this->candidates($body);
        self::assertSame([796, 6, 7], array_column($candidates, 'external_id'));
        // The series still gets its people; the issues go without theirs, and nobody was asked for them.
        self::assertSame(['Bill Finger'], $candidates[0]['writers']);
        self::assertSame([], $candidates[1]['writers']);
        self::assertSame([], $candidates[2]['writers']);
        self::assertSame(['/api/volume/4050-796/'], FakeComicVineApi::detailPaths());

        // And the search itself, and the details of series, are not touched by it.
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume(797)]);
        FakeComicVineApi::detailIs('volume', 797, FakeComicVineApi::credits());
        $second = $this->search($client, $ada, 'another comic');
        self::assertResponseStatusCodeSame(200);
        self::assertSame(['Bill Finger'], $this->candidates($second)[0]['writers']);
    }

    public function testTheLimitOfSeriesDetailsDoesNotTakeTheSearchOrTheIssuesDown(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $limiter = self::getContainer()->get(RequestLimiter::class);
        for ($i = 0; $i < 150; ++$i) {
            $limiter->consumeComicVineCall(ComicVineResource::Volume);
        }
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume(796), FakeComicVineApi::issue(6)]);
        FakeComicVineApi::detailIs('issue', 6, FakeComicVineApi::credits());

        $candidates = $this->candidates($this->search($client, $ada, 'comic'));

        self::assertResponseStatusCodeSame(200);
        self::assertSame([], $candidates[0]['writers']);
        self::assertSame(['Bill Finger'], $candidates[1]['writers']);
        self::assertSame(['/api/issue/4000-6/'], FakeComicVineApi::detailPaths());
    }

    // --- The cache's age ------------------------------------------------------------------------------------------------

    public function testASearchIsKeptAWeekAndADetailThirtyDays(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $now = new \DateTimeImmutable();
        $this->store('comic_vine_search', ComicVineSearch::fingerprint('comic'), true, ['hits' => [['resource' => 'volume', 'external_id' => 796, 'title' => 'Batman', 'series' => null, 'number' => null, 'year' => 1940, 'publisher' => 'DC', 'cover_url' => null]]], $now->modify('-6 days'));
        $this->store('comic_vine_volume', '796', true, ['writers' => ['Cached Writer'], 'artists' => []], $now->modify('-29 days'));

        $body = $this->search($client, $ada, 'comic');

        self::assertSame(0, FakeComicVineApi::calls());
        self::assertSame(['Cached Writer'], $this->candidates($body)[0]['writers']);

        $this->entityManager()->createQuery(\sprintf('UPDATE %s e SET e.fetchedAt = :at WHERE e.provider = :provider', MetadataCacheEntry::class))
            ->setParameter('at', $now->modify('-8 days'))->setParameter('provider', 'comic_vine_search')->execute();
        $this->entityManager()->createQuery(\sprintf('UPDATE %s e SET e.fetchedAt = :at WHERE e.provider = :provider', MetadataCacheEntry::class))
            ->setParameter('at', $now->modify('-31 days'))->setParameter('provider', 'comic_vine_volume')->execute();
        $this->entityManager()->clear();
        FakeComicVineApi::searchFinds([FakeComicVineApi::volume()]);
        FakeComicVineApi::detailIs('volume', 796, FakeComicVineApi::credits());

        $again = $this->search($client, $ada, 'comic');

        self::assertSame(2, FakeComicVineApi::calls());
        self::assertSame(['Bill Finger'], $this->candidates($again)[0]['writers']);
    }

    // --- helpers ------------------------------------------------------------------------------------------------------

    private function world(): KernelBrowser
    {
        $_ENV['COMIC_VINE_API_KEY'] = $_SERVER['COMIC_VINE_API_KEY'] = self::KEY;
        self::ensureKernelShutdown();
        $client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();
        $this->clearBrake();
        FakeComicVineApi::reset();
        RecordingLogger::reset();

        return $client;
    }

    private function clearBrake(): void
    {
        self::getContainer()->get('cache.metadata')->clear();
    }

    /** @return array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private function account(KernelBrowser $client, string $email): array
    {
        return $this->signInNewDevice($client, $email);
    }

    /** @return list<mixed> what `GET /v1/server` says it looks up */
    private function offered(KernelBrowser $client): array
    {
        $metadata = $this->decode($client)['metadata'] ?? [];

        return \is_array($metadata) ? array_values($metadata) : [];
    }

    private function payload(string $query): string
    {
        return json_encode(['query' => $query], \JSON_THROW_ON_ERROR | \JSON_INVALID_UTF8_SUBSTITUTE);
    }

    /**
     * @param array{headers: array<string, string>} $session
     *
     * @return array<string, mixed>
     */
    private function search(KernelBrowser $client, array $session, string $query): array
    {
        $this->raw($client, $session, $query);

        return $this->decode($client);
    }

    /** @param array{headers: array<string, string>} $session */
    private function raw(KernelBrowser $client, array $session, string $query): string
    {
        $client->request('POST', '/v1/metadata/comic-vine/search', server: $session['headers'] + ['CONTENT_TYPE' => 'application/json'], content: $this->payload($query));

        return (string) $client->getResponse()->getContent();
    }

    /**
     * @param array<string, mixed> $body
     *
     * @return list<array<string, mixed>>
     */
    private function candidates(array $body): array
    {
        $list = $body['candidates'] ?? [];
        $out = [];
        foreach (\is_array($list) ? $list : [] as $candidate) {
            $row = [];
            foreach (\is_array($candidate) ? $candidate : [] as $key => $value) {
                $row[(string) $key] = $value;
            }
            $out[] = $row;
        }

        return $out;
    }

    /** @return array<string, mixed> */
    private function decode(KernelBrowser $client): array
    {
        $decoded = json_decode((string) $client->getResponse()->getContent(), true);
        if (!\is_array($decoded)) {
            return [];
        }
        $body = [];
        foreach ($decoded as $key => $value) {
            $body[(string) $key] = $value;
        }

        return $body;
    }

    private function entityManager(): EntityManagerInterface
    {
        return self::getContainer()->get(EntityManagerInterface::class);
    }

    private function user(string $email): User
    {
        return $this->entityManager()->getRepository(User::class)->findOneBy(['email' => $email]) ?? self::fail('No such user.');
    }

    private function numberFrom(string $sql): int
    {
        $count = $this->entityManager()->getConnection()->fetchOne($sql);

        return is_numeric($count) ? (int) $count : self::fail('Not a count.');
    }

    private function cacheRows(): int
    {
        return $this->numberFrom('SELECT COUNT(*) FROM metadata_cache');
    }

    private function detailRows(): int
    {
        return $this->numberFrom("SELECT COUNT(*) FROM metadata_cache WHERE provider IN ('comic_vine_volume', 'comic_vine_issue')");
    }

    /** @return list<string> */
    private function codes(): array
    {
        return $this->strings($this->entityManager()->getConnection()->fetchFirstColumn('SELECT code FROM metadata_cache ORDER BY code'));
    }

    /**
     * @param list<mixed> $values
     *
     * @return list<string>
     */
    private function strings(array $values): array
    {
        return array_map(static fn (mixed $value): string => \is_string($value) ? $value : self::fail('Not a string.'), $values);
    }

    /** @return list<string> */
    private function cacheColumns(): array
    {
        return $this->strings($this->entityManager()->getConnection()->fetchFirstColumn(
            "SELECT column_name FROM information_schema.columns WHERE table_name = 'metadata_cache' ORDER BY ordinal_position"
        ));
    }

    /** @return list<string> */
    private function allCacheRowsAsText(): array
    {
        $rows = $this->entityManager()->getConnection()->fetchAllAssociative('SELECT * FROM metadata_cache');

        return array_map(static fn (array $row): string => json_encode($row, \JSON_THROW_ON_ERROR), $rows);
    }

    /** @param array<string, mixed> $payload */
    private function store(string $provider, string $code, bool $found, array $payload, \DateTimeImmutable $at): void
    {
        $this->entityManager()->persist(new MetadataCacheEntry($provider, $code, $found, $payload, $at));
        $this->entityManager()->flush();
    }
}
