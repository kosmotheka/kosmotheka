<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Entity\ActivityEntry;
use App\Entity\MetadataCacheEntry;
use App\Entity\User;
use App\Metadata\RawgSearch;
use App\Security\RequestLimiter;
use App\Tests\Support\DeviceAuthTestTrait;
use App\Tests\Support\FakeRawgApi;
use App\Tests\Support\RecordingLogger;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\HttpClient\Exception\TransportException;

/**
 * Searching video games by title in RAWG through the server (7.8, D62). Two things make this one delicate: the title a
 * person looks for (it never travels in an address of ours, is never kept in the clear and never reaches a log), and
 * RAWG's key, which RAWG only takes in the address of the outgoing call, where every exception, log line and response
 * could carry it. Hostile cases first.
 */
final class RawgSearchTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string KEY = 'SENTINEL-rawg-key-5d2e';
    private const string SECRET_TITLE = 'zxqv private game';

    protected function tearDown(): void
    {
        $_ENV['RAWG_API_KEY'] = $_SERVER['RAWG_API_KEY'] = '';
        FakeRawgApi::reset();
        RecordingLogger::reset();
        parent::tearDown();
    }

    // --- The way -----------------------------------------------------------------------------------------------------

    public function testASearchGivesTheFixedShapeWithTheDetailOfTheFirstThree(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeRawgApi::searchFinds([FakeRawgApi::game(1, 'One'), FakeRawgApi::game(2, 'Two'), FakeRawgApi::game(3, 'Three'), FakeRawgApi::game(4, 'Four'), FakeRawgApi::game(5, 'Five')]);
        foreach ([1, 2, 3] as $id) {
            FakeRawgApi::detailIs($id, FakeRawgApi::gameDetail());
        }

        $body = $this->search($client, $ada, 'gta');

        self::assertResponseStatusCodeSame(200);
        self::assertSame(['candidates'], array_keys($body));
        $candidates = $this->candidates($body);
        self::assertCount(5, $candidates);
        self::assertSame([
            'external_id' => 1,
            'title' => 'One',
            'year' => 2013,
            'cover_url' => 'https://media.rawg.io/media/games/456/456dea5e1c7e3cd07060c14e96612001.jpg',
            'platforms' => ['PC', 'PlayStation 5'],
            'developers' => ['Rockstar North'],
            'publishers' => ['Rockstar Games'],
        ], $candidates[0]);
        self::assertSame([], $candidates[3]['developers']);
        self::assertSame([], $candidates[4]['publishers']);
        // One search, and the detail of the first three only.
        self::assertSame(1, FakeRawgApi::searchCalls());
        self::assertSame(['/api/games/1', '/api/games/2', '/api/games/3'], array_map(static fn (array $r): string => $r['path'], \array_slice(FakeRawgApi::$requests, 1)));
        $search = FakeRawgApi::$requests[0];
        self::assertSame('GET', $search['method']);
        self::assertSame('gta', $search['query']['search'] ?? null);
        self::assertSame('8', $search['query']['page_size'] ?? null);
        self::assertSame(self::KEY, $search['query']['key'] ?? null);
        self::assertStringStartsWith('https://api.rawg.io/api/games?', $search['url']);
    }

    public function testNothingFoundIsAnAnswerAndIsRemembered(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeRawgApi::searchFinds([]);

        $first = $this->search($client, $ada, 'qwertyuiop');
        $second = $this->search($client, $ada, 'qwertyuiop');

        self::assertResponseStatusCodeSame(200);
        self::assertSame(['candidates' => []], $first);
        self::assertSame($first, $second);
        self::assertSame(1, FakeRawgApi::calls());
    }

    public function testTheServerListsRawgOnlyWithAKey(): void
    {
        $client = $this->world();
        $client->request('GET', '/v1/server');
        self::assertContains('rawg', $this->offered($client));

        $_ENV['RAWG_API_KEY'] = $_SERVER['RAWG_API_KEY'] = '';
        self::ensureKernelShutdown();
        $client = self::createClient();
        $client->request('GET', '/v1/server');
        self::assertNotContains('rawg', $this->offered($client));
    }

    public function testWithoutAKeyTheRouteIsUnavailableAndNobodyIsAsked(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $_ENV['RAWG_API_KEY'] = $_SERVER['RAWG_API_KEY'] = '';
        self::ensureKernelShutdown();
        $client = self::createClient();

        $body = $this->search($client, $ada, 'gta');

        self::assertResponseStatusCodeSame(404);
        self::assertSame('metadata_unavailable', $body['error'] ?? null);
        self::assertSame(0, FakeRawgApi::calls());
    }

    // --- 1. Who may ask -----------------------------------------------------------------------------------------------

    public function testWithoutASessionThereIsNoAnswerAndNoCall(): void
    {
        $client = $this->world();

        $client->request('POST', '/v1/metadata/rawg/search', server: ['CONTENT_TYPE' => 'application/json'], content: $this->payload('gta'));

        self::assertResponseStatusCodeSame(401);
        self::assertSame(0, FakeRawgApi::calls());
    }

    public function testAnInvalidTokenGetsNoAnswerAndNoCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        $this->search($client, ['headers' => $this->deviceHeaders($ada['device_id'], 'nope')], 'gta');

        self::assertResponseStatusCodeSame(401);
        self::assertSame(0, FakeRawgApi::calls());
    }

    public function testAnAccountNoLongerConfirmedGetsNoAnswerAndNoCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $this->entityManager()->createQuery(\sprintf('UPDATE %s u SET u.confirmedAt = NULL', User::class))->execute();

        $body = $this->search($client, $ada, 'gta');

        self::assertResponseStatusCodeSame(403);
        self::assertSame('account_not_confirmed', $body['error'] ?? null);
        self::assertSame(0, FakeRawgApi::calls());
    }

    public function testABlockedAccountGetsNoAnswerAndNoCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $this->user('ada@example.com')->block(new \DateTimeImmutable());
        $this->entityManager()->flush();

        $body = $this->search($client, $ada, 'gta');

        self::assertResponseStatusCodeSame(403);
        self::assertSame('account_blocked', $body['error'] ?? null);
        self::assertSame(0, FakeRawgApi::calls());
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
        yield 'a line break' => ['{"query":"gt\na"}'];
        yield 'a tab' => ['{"query":"gt\ta"}'];
        yield 'a null byte' => ['{"query":"gt\u0000a"}'];
        yield 'a control character' => ['{"query":"gt\u0007a"}'];
        yield 'more fields' => ['{"query":"gta","kind":"videogame"}'];
        yield 'a key' => ['{"query":"gta","key":"x"}'];
        yield 'broken json' => ['{"query":"gta"'];
        yield 'a list' => ['["gta"]'];
        yield 'a string' => ['"gta"'];
        yield 'nothing' => [''];
    }

    #[DataProvider('badBodies')]
    public function testABodyThatIsNotTheOneAskedIsRefusedWithoutRawgAndWithoutARow(string $body): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        $client->request('POST', '/v1/metadata/rawg/search', server: $ada['headers'] + ['CONTENT_TYPE' => 'application/json'], content: $body);

        self::assertGreaterThanOrEqual(400, $client->getResponse()->getStatusCode());
        self::assertLessThan(500, $client->getResponse()->getStatusCode());
        self::assertSame(0, FakeRawgApi::calls());
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

        self::assertSame(0, FakeRawgApi::calls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testSurroundingBlanksAndRepeatedSpacesDoNotChangeWhatIsAsked(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeRawgApi::searchFinds([]);

        $this->search($client, $ada, "  Grand   Theft \u{a0}");

        self::assertSame('grand theft', FakeRawgApi::$requests[0]['query']['search'] ?? null);
    }

    // --- 3. A title that tries to be more than a title ---------------------------------------------------------------

    public function testATitleWithSymbolsTravelsAsOneEncodedParameter(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        foreach (['a&key=other', 'what? #1', 'a/../../b', '100%00 sure', 'x&page_size=500&search=y', 'ñ ü ゲーム'] as $title) {
            FakeRawgApi::searchFinds([]);
            $this->search($client, $ada, $title);
            self::assertResponseStatusCodeSame(200, $title);
            $request = FakeRawgApi::$requests[\count(FakeRawgApi::$requests) - 1];
            self::assertSame('/api/games', $request['path'], $title);
            self::assertSame(mb_strtolower($title), $request['query']['search'] ?? null, $title);
            self::assertEqualsCanonicalizing(['search', 'page_size', 'key'], array_keys($request['query']), $title);
            self::assertSame('8', $request['query']['page_size']);
            self::assertSame(self::KEY, $request['query']['key']);
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
        $withKey = 'https://api.rawg.io/api/games?search=x&key='.self::KEY;

        // RAWG echoing the address in its errors, every status that is not a success.
        foreach ([401, 403, 429, 404, 400, 500, 502] as $status) {
            $times = $status >= 500 ? 3 : 1;
            for ($i = 0; $i < $times; ++$i) {
                FakeRawgApi::searchAnswers($status, '{"detail":"bad request '.$withKey.'"}');
            }
            $this->clearBrake();
            $seen[] = $this->raw($client, $ada, 'query '.++$n);
        }
        // The http client failing the way it does: its messages carry the address.
        foreach ([
            new TransportException('Could not resolve host for "'.$withKey.'"'),
            new TransportException('Idle timeout reached for "'.$withKey.'"'),
            new \RuntimeException('Failed '.$withKey, previous: new TransportException($withKey)),
        ] as $error) {
            FakeRawgApi::searchFails($error);
            $this->clearBrake();
            $seen[] = $this->raw($client, $ada, 'query '.++$n);
        }
        // Rubbish that is not JSON, and JSON that echoes it.
        FakeRawgApi::searchAnswers(200, '<html>'.$withKey.'</html>');
        $this->clearBrake();
        $seen[] = $this->raw($client, $ada, 'query '.++$n);
        FakeRawgApi::searchFinds([FakeRawgApi::game(7, 'Echo '.$withKey)]);
        FakeRawgApi::detailAnswers(7, 500, $withKey);
        $this->clearBrake();
        $seen[] = $this->raw($client, $ada, 'query '.++$n);
        FakeRawgApi::searchFinds([FakeRawgApi::game(8)]);
        FakeRawgApi::detailFails(8, new TransportException('timeout for "'.$withKey.'"'));
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
        self::assertStringNotContainsString('key=', RecordingLogger::everything());
    }

    public function testAnAnswerThatBreaksWhileItIsReadLeaksNothingEither(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $withKey = 'https://api.rawg.io/api/games?search=x&key='.self::KEY;
        FakeRawgApi::searchBreaksWhileRead(new TransportException('Connection reset for "'.$withKey.'"'));
        $first = $this->raw($client, $ada, 'one');
        $firstStatus = $client->getResponse()->getStatusCode();
        FakeRawgApi::searchFinds([FakeRawgApi::game(9)]);
        FakeRawgApi::detailBreaksWhileRead(9, new TransportException('Connection reset for "'.$withKey.'"'));
        $second = $this->raw($client, $ada, 'two');

        self::assertSame(502, $firstStatus);
        self::assertSame(200, $client->getResponse()->getStatusCode());
        self::assertStringNotContainsString(self::KEY, $first.$second);
        self::assertStringNotContainsString(self::KEY, RecordingLogger::everything());
        self::assertStringNotContainsString(self::KEY, implode(' ', $this->allCacheRowsAsText()));
    }

    // --- 5, 12 and 13. A RAWG that sends rubbish ----------------------------------------------------------------------

    public function testAThousandResultsReachTheAppAsEightAndOnlyThreeDetailsAreAsked(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $results = [];
        for ($i = 1; $i <= 1000; ++$i) {
            $results[] = FakeRawgApi::game($i, 'G'.$i);
        }
        FakeRawgApi::searchFinds($results);

        $body = $this->search($client, $ada, 'game');

        self::assertResponseStatusCodeSame(200);
        self::assertSame(range(1, 8), array_column($this->candidates($body), 'external_id'));
        self::assertSame(3, FakeRawgApi::detailCalls());
    }

    public function testFieldsOfTheWrongTypeAndHugeOnesAreCutOrDropped(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $platforms = [];
        for ($i = 0; $i < 100; ++$i) {
            $platforms[] = ['platform' => ['id' => $i, 'name' => 'Platform '.$i]];
        }
        $platforms[] = ['platform' => ['name' => str_repeat('P', 5000)]];
        $platforms[] = ['platform' => 'x'];
        $platforms[] = 'y';
        FakeRawgApi::searchFinds([
            ['id' => 7, 'name' => str_repeat('T', 30_000), 'released' => 'long ago', 'background_image' => 42, 'platforms' => $platforms, 'extra' => str_repeat('o', 1000)],
            ['id' => 'abc', 'name' => 'string id'],
            ['id' => [1], 'name' => 'array id'],
            ['id' => -4, 'name' => 'negative id'],
            ['id' => 1.5, 'name' => 'float id'],
            ['id' => 99_999_999_999_999, 'name' => 'too big id'],
            ['id' => 0, 'name' => 'zero id'],
            ['name' => 'no id'],
            'not an object',
            ['id' => 9, 'name' => ['x']],
        ]);
        FakeRawgApi::detailIs(7, ['developers' => array_merge(array_fill(0, 100, ['name' => 'Dev']), [['name' => str_repeat('N', 5000)], 'x', ['name' => ['y']]]), 'publishers' => 'nobody']);

        $body = $this->search($client, $ada, 'game');

        self::assertResponseStatusCodeSame(200);
        $candidates = $this->candidates($body);
        self::assertSame([7], array_column($candidates, 'external_id'));
        $candidate = $candidates[0];
        self::assertIsString($candidate['title']);
        self::assertSame(300, mb_strlen($candidate['title']));
        self::assertNull($candidate['year']);
        self::assertNull($candidate['cover_url']);
        self::assertIsArray($candidate['platforms']);
        self::assertCount(10, $candidate['platforms']);
        self::assertSame('Platform 0', $candidate['platforms'][0]);
        self::assertIsArray($candidate['developers']);
        self::assertLessThanOrEqual(5, \count($candidate['developers']));
        self::assertSame([], $candidate['publishers']);
        // Only the id RAWG itself gave, as a number, is ever put in an address.
        self::assertSame(['/api/games', '/api/games/7'], array_map(static fn (array $r): string => $r['path'], FakeRawgApi::$requests));
    }

    public function testNamesOfHugeSizeAreCut(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeRawgApi::searchFinds([['id' => 7, 'name' => 'G', 'platforms' => [['platform' => ['name' => str_repeat('P', 5000)]]]]]);
        FakeRawgApi::detailIs(7, ['developers' => [['name' => str_repeat('N', 5000)]], 'publishers' => [['name' => str_repeat('M', 5000)]]]);

        $candidate = $this->candidates($this->search($client, $ada, 'game'))[0];

        self::assertSame([str_repeat('P', 100)], $candidate['platforms']);
        self::assertSame([str_repeat('N', 200)], $candidate['developers']);
        self::assertSame([str_repeat('M', 200)], $candidate['publishers']);
    }

    /** @return iterable<string, array{mixed}> */
    public static function covers(): iterable
    {
        yield 'a parent directory' => ['https://media.rawg.io/media/../x.jpg'];
        yield 'a parent directory encoded' => ['https://media.rawg.io/media/%2e%2e/x.jpg'];
        yield 'another site' => ['https://evil.example/media/games/x.jpg'];
        yield 'a look-alike host' => ['https://media.rawg.io.evil.example/media/x.jpg'];
        yield 'a user in the address' => ['https://media.rawg.io@evil.example/media/x.jpg'];
        yield 'a user before the host' => ['https://user@media.rawg.io/media/x.jpg'];
        yield 'a port' => ['https://media.rawg.io:8443/media/x.jpg'];
        yield 'http' => ['http://media.rawg.io/media/x.jpg'];
        yield 'a protocol-relative one' => ['//media.rawg.io/media/x.jpg'];
        yield 'another directory' => ['https://media.rawg.io/other/x.jpg'];
        yield 'not an image' => ['https://media.rawg.io/media/x.svg'];
        yield 'a script' => ['https://media.rawg.io/media/x.html'];
        yield 'no extension' => ['https://media.rawg.io/media/x'];
        yield 'a query' => ['https://media.rawg.io/media/x.jpg?evil=1'];
        yield 'a fragment' => ['https://media.rawg.io/media/x.jpg#evil'];
        yield 'another address inside' => ['https://media.rawg.io/media/https://evil.example/x.jpg'];
        yield 'a line break' => ["https://media.rawg.io/media/x.jpg\n"];
        yield 'a space' => ['https://media.rawg.io/media/x y.jpg'];
        yield 'an empty segment' => ['https://media.rawg.io/media//x.jpg'];
        yield 'very long' => ['https://media.rawg.io/media/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.jpg'];
        yield 'an array' => [['https://media.rawg.io/media/x.jpg']];
    }

    #[DataProvider('covers')]
    public function testACoverThatIsNotAPlainImageOfTheirHostIsDropped(mixed $cover): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeRawgApi::searchFinds([['id' => 5, 'name' => 'G', 'background_image' => $cover]]);

        $body = $this->search($client, $ada, 'game');

        self::assertResponseStatusCodeSame(200);
        self::assertNull($this->candidates($body)[0]['cover_url']);
    }

    /** @return iterable<string, array{string}> */
    public static function goodCovers(): iterable
    {
        yield 'jpg' => ['https://media.rawg.io/media/games/456/456dea5e1c7e3cd07060c14e96612001.jpg'];
        yield 'jpeg' => ['https://media.rawg.io/media/games/456/a.jpeg'];
        yield 'png' => ['https://media.rawg.io/media/screenshots/a_b-c.png'];
        yield 'webp' => ['https://media.rawg.io/media/crop/600/400/games/456/a.webp'];
    }

    #[DataProvider('goodCovers')]
    public function testACoverOfTheirImageHostIsKept(string $cover): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeRawgApi::searchFinds([['id' => 5, 'name' => 'G', 'background_image' => $cover]]);

        $body = $this->search($client, $ada, 'game');

        self::assertSame($cover, $this->candidates($body)[0]['cover_url']);
    }

    public function testAnswersThatAreNotUsableAreAFailureAndLeaveNoRow(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $n = 0;

        foreach (['<html>hello</html>', '[1,2,3', 'null', '"text"', '{"results":"no"}', '{}', str_repeat('{"a":1,', 400_000)] as $nonsense) {
            FakeRawgApi::searchAnswers(200, $nonsense);
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
        FakeRawgApi::searchFinds([FakeRawgApi::game(1, str_repeat('T', 1_000_000))]);

        $body = $this->search($client, $ada, 'game');

        self::assertResponseStatusCodeSame(502);
        self::assertSame('metadata_failed', $body['error'] ?? null);
        self::assertSame(0, $this->cacheRows());
    }

    // --- 6. RAWG says no ----------------------------------------------------------------------------------------------

    /** @return iterable<string, array{int}> */
    public static function refusals(): iterable
    {
        yield 'too many requests' => [429];
        yield 'unauthorised' => [401];
        yield 'forbidden' => [403];
    }

    #[DataProvider('refusals')]
    public function testRawgRefusingIsReportedAndNothingIsStored(int $status): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeRawgApi::searchAnswers($status, '{"detail":"no"}');

        $body = $this->search($client, $ada, 'game');

        self::assertResponseStatusCodeSame(503);
        self::assertSame('metadata_quota_exhausted', $body['error'] ?? null);
        self::assertSame(0, $this->cacheRows());
    }

    public function testAfterARefusalTheServerStopsAskingRawgForAWhile(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeRawgApi::searchAnswers(429, '{}');

        $this->search($client, $ada, 'game');
        for ($try = 0; $try < 5; ++$try) {
            $body = $this->search($client, $ada, 'other '.$try);
            self::assertSame('metadata_quota_exhausted', $body['error'] ?? null);
        }

        self::assertSame(1, FakeRawgApi::calls());
        self::assertGreaterThan(0, (int) $client->getResponse()->headers->get('Retry-After'));

        $this->clearBrake();
        FakeRawgApi::searchFinds([]);
        $this->search($client, $ada, 'game');
        self::assertResponseStatusCodeSame(200);
        self::assertSame(2, FakeRawgApi::calls());
    }

    public function testAServerErrorIsTriedAgainAndThenAnswers(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeRawgApi::searchAnswers(503, 'down');
        FakeRawgApi::searchFinds([FakeRawgApi::game()]);
        FakeRawgApi::detailIs(3498, FakeRawgApi::gameDetail());

        $body = $this->search($client, $ada, 'game');

        self::assertResponseStatusCodeSame(200);
        self::assertCount(1, $this->candidates($body));
        self::assertSame(2, FakeRawgApi::searchCalls());
    }

    public function testAServerErrorThatKeepsComingStopsAtThreeTriesAndLeavesNoRow(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        for ($i = 0; $i < 5; ++$i) {
            FakeRawgApi::searchAnswers(500, 'oops');
        }

        $body = $this->search($client, $ada, 'game');

        self::assertResponseStatusCodeSame(502);
        self::assertSame('metadata_failed', $body['error'] ?? null);
        self::assertSame(3, FakeRawgApi::searchCalls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testASlowRawgIsNotAskedAgain(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeRawgApi::searchFails(new TransportException('Idle timeout reached'));

        $this->search($client, $ada, 'game');

        self::assertResponseStatusCodeSame(502);
        self::assertSame(1, FakeRawgApi::calls());
    }

    // --- 7. A detail that fails ---------------------------------------------------------------------------------------

    public function testADetailThatFailsLeavesThatCandidateIncompleteAndTheSearchAnswers(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeRawgApi::searchFinds([FakeRawgApi::game(1, 'One'), FakeRawgApi::game(2, 'Two'), FakeRawgApi::game(3, 'Three')]);
        FakeRawgApi::detailIs(1, FakeRawgApi::gameDetail());
        FakeRawgApi::detailAnswers(2, 500, 'oops');
        FakeRawgApi::detailFails(3, new TransportException('Idle timeout reached'));

        $body = $this->search($client, $ada, 'game');

        self::assertResponseStatusCodeSame(200);
        $candidates = $this->candidates($body);
        self::assertCount(3, $candidates);
        self::assertSame(['Rockstar North'], $candidates[0]['developers']);
        foreach (\array_slice($candidates, 1) as $candidate) {
            self::assertSame([], $candidate['developers']);
            self::assertSame([], $candidate['publishers']);
            self::assertNotNull($candidate['title']);
        }
    }

    public function testADetailRefusedOnQuotaPausesRawgForTheNextSearch(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeRawgApi::searchFinds([FakeRawgApi::game()]);
        FakeRawgApi::detailAnswers(3498, 429, '{}');

        $this->search($client, $ada, 'game');
        self::assertResponseStatusCodeSame(200);
        $calls = FakeRawgApi::calls();
        $body = $this->search($client, $ada, 'something else');

        self::assertResponseStatusCodeSame(503);
        self::assertSame('metadata_quota_exhausted', $body['error'] ?? null);
        self::assertSame($calls, FakeRawgApi::calls());
    }

    public function testADetailThatFailedIsNotRememberedAndIsAskedAgainNextTime(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeRawgApi::searchFinds([FakeRawgApi::game()]);
        FakeRawgApi::detailAnswers(3498, 500, 'oops');

        $this->search($client, $ada, 'game');
        self::assertSame(0, $this->detailRows());
        FakeRawgApi::detailIs(3498, FakeRawgApi::gameDetail());
        $second = $this->search($client, $ada, 'game');

        self::assertSame(['Rockstar North'], $this->candidates($second)[0]['developers']);
        self::assertSame(1, FakeRawgApi::searchCalls());
        self::assertSame(1, $this->detailRows());
    }

    // --- 8. Nobody is remembered --------------------------------------------------------------------------------------

    public function testTwoAccountsLookingForTheSameCostOneRoundOfCallsAndNoRowSaysWhoOrWhat(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        FakeRawgApi::searchFinds([FakeRawgApi::game()]);
        FakeRawgApi::detailIs(3498, FakeRawgApi::gameDetail());
        $activity = $this->entityManager()->getRepository(ActivityEntry::class)->count([]);

        $this->search($client, $ada, self::SECRET_TITLE);
        $this->search($client, $grace, '  '.mb_strtoupper(self::SECRET_TITLE).' ');

        self::assertSame(2, FakeRawgApi::calls());
        self::assertSame(2, $this->cacheRows());
        self::assertSame(['id', 'provider', 'code', 'found', 'payload', 'fetched_at'], $this->cacheColumns());
        $rows = implode(' ', $this->allCacheRowsAsText());
        self::assertStringNotContainsString('ada@example.com', $rows);
        self::assertStringNotContainsString('grace@example.com', $rows);
        self::assertStringNotContainsString($this->user('ada@example.com')->getId()->toRfc4122(), $rows);
        self::assertStringNotContainsStringIgnoringCase('zxqv', $rows);
        self::assertStringNotContainsStringIgnoringCase('private game', $rows);
        self::assertContains(RawgSearch::fingerprint(self::SECRET_TITLE), $this->codes());
        self::assertSame($activity, $this->entityManager()->getRepository(ActivityEntry::class)->count([]));
    }

    public function testTheSearchedTitleReachesNoLogEvenWhenSomethingGoesWrong(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeRawgApi::searchAnswers(429, '{}');
        $this->search($client, $ada, self::SECRET_TITLE);
        $this->clearBrake();
        FakeRawgApi::searchAnswers(500, 'x');
        FakeRawgApi::searchAnswers(500, 'x');
        FakeRawgApi::searchAnswers(500, 'x');
        $this->search($client, $ada, self::SECRET_TITLE.' two');
        $this->search($client, $ada, self::SECRET_TITLE."\n");
        $client->request('POST', '/v1/metadata/rawg/search', server: $ada['headers'] + ['CONTENT_TYPE' => 'application/json'], content: '{"query":"'.self::SECRET_TITLE.'"');
        $client->request('POST', '/v1/metadata/rawg/search', server: $ada['headers'] + ['CONTENT_TYPE' => 'application/json'], content: '{"query":"'.self::SECRET_TITLE.'","extra":1}');

        self::assertNotSame('', RecordingLogger::everything(), 'the logger must be the one the test reads');
        self::assertStringNotContainsStringIgnoringCase('zxqv', RecordingLogger::everything());
        self::assertStringNotContainsStringIgnoringCase('private game', RecordingLogger::everything());
    }

    // --- 9. Limits ----------------------------------------------------------------------------------------------------

    public function testAnAccountPastItsSharedLimitIsCutAndTheOthersAreNot(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        $limiter = self::getContainer()->get(RequestLimiter::class);
        for ($i = 0; $i < 599; ++$i) {
            $limiter->consumeMetadata($this->user('ada@example.com')->getId()->toRfc4122());
        }
        FakeRawgApi::searchFinds([]);
        FakeRawgApi::searchFinds([]);

        $this->search($client, $ada, 'game');
        self::assertResponseStatusCodeSame(200);
        $body = $this->search($client, $ada, 'game');
        self::assertResponseStatusCodeSame(429);
        self::assertSame('too_many_requests', $body['error'] ?? null);

        $this->search($client, $grace, 'game');
        self::assertResponseStatusCodeSame(200);
    }

    public function testWhenTheServersDayIsSpentTheCacheKeepsServing(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeRawgApi::searchFinds([FakeRawgApi::game()]);
        FakeRawgApi::detailIs(3498, FakeRawgApi::gameDetail());
        $this->search($client, $ada, 'game');
        $limiter = self::getContainer()->get(RequestLimiter::class);
        for ($i = 0; $i < 598; ++$i) {
            $limiter->consumeRawgCall(); // with the two calls made above, the 600 of the day
        }

        $cached = $this->search($client, $ada, 'game');
        self::assertResponseStatusCodeSame(200);
        self::assertSame('Grand Theft Auto V', $this->candidates($cached)[0]['title']);
        self::assertSame(['Rockstar North'], $this->candidates($cached)[0]['developers']);

        $other = $this->search($client, $ada, 'other');
        self::assertResponseStatusCodeSame(503);
        self::assertSame('metadata_quota_exhausted', $other['error'] ?? null);
        self::assertSame(2, FakeRawgApi::calls());
    }

    public function testTheDayRunningOutWhileTheDetailsAreAskedLeavesTheRestIncomplete(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $limiter = self::getContainer()->get(RequestLimiter::class);
        for ($i = 0; $i < 598; ++$i) {
            $limiter->consumeRawgCall(); // the search and one detail are what is left
        }
        FakeRawgApi::searchFinds([FakeRawgApi::game(1, 'One'), FakeRawgApi::game(2, 'Two'), FakeRawgApi::game(3, 'Three')]);
        foreach ([1, 2, 3] as $id) {
            FakeRawgApi::detailIs($id, FakeRawgApi::gameDetail());
        }

        $body = $this->search($client, $ada, 'game');

        self::assertResponseStatusCodeSame(200);
        self::assertCount(3, $this->candidates($body));
        self::assertSame(2, FakeRawgApi::calls());
    }

    // --- The cache's age ------------------------------------------------------------------------------------------------

    /** SEG-057: what comes back from the cache passes the same cover check as what came from RAWG. */
    public function testACoverInTheCacheThatRawgCouldNotHaveSentIsDropped(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $this->store('rawg_search', RawgSearch::fingerprint('game'), true, ['hits' => [['external_id' => 3498, 'title' => 'GTA', 'year' => 2013, 'cover_url' => 'https://evil.example/x.jpg', 'platforms' => []]]], new \DateTimeImmutable());
        $this->store('rawg_game', '3498', true, ['developers' => [], 'publishers' => []], new \DateTimeImmutable());

        $body = $this->search($client, $ada, 'game');

        self::assertSame(0, FakeRawgApi::calls());
        self::assertNull($this->candidates($body)[0]['cover_url']);
    }

    public function testASearchIsKeptAWeekAndADetailThirtyDays(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $now = new \DateTimeImmutable();
        $this->store('rawg_search', RawgSearch::fingerprint('game'), true, ['hits' => [['external_id' => 3498, 'title' => 'GTA', 'year' => 2013, 'cover_url' => null, 'platforms' => ['PC']]]], $now->modify('-6 days'));
        $this->store('rawg_game', '3498', true, ['developers' => ['Cached Dev'], 'publishers' => []], $now->modify('-29 days'));

        $body = $this->search($client, $ada, 'game');

        self::assertSame(0, FakeRawgApi::calls());
        self::assertSame(['Cached Dev'], $this->candidates($body)[0]['developers']);

        $this->entityManager()->createQuery(\sprintf('UPDATE %s e SET e.fetchedAt = :at WHERE e.provider = :provider', MetadataCacheEntry::class))
            ->setParameter('at', $now->modify('-8 days'))->setParameter('provider', 'rawg_search')->execute();
        $this->entityManager()->createQuery(\sprintf('UPDATE %s e SET e.fetchedAt = :at WHERE e.provider = :provider', MetadataCacheEntry::class))
            ->setParameter('at', $now->modify('-31 days'))->setParameter('provider', 'rawg_game')->execute();
        $this->entityManager()->clear();
        FakeRawgApi::searchFinds([FakeRawgApi::game()]);
        FakeRawgApi::detailIs(3498, FakeRawgApi::gameDetail());

        $again = $this->search($client, $ada, 'game');

        self::assertSame(2, FakeRawgApi::calls());
        self::assertSame(['Rockstar North'], $this->candidates($again)[0]['developers']);
    }

    // --- helpers ------------------------------------------------------------------------------------------------------

    private function world(): KernelBrowser
    {
        $_ENV['RAWG_API_KEY'] = $_SERVER['RAWG_API_KEY'] = self::KEY;
        self::ensureKernelShutdown();
        $client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();
        $this->clearBrake();
        FakeRawgApi::reset();
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
        $client->request('POST', '/v1/metadata/rawg/search', server: $session['headers'] + ['CONTENT_TYPE' => 'application/json'], content: $this->payload($query));

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
        return $this->numberFrom("SELECT COUNT(*) FROM metadata_cache WHERE provider = 'rawg_game'");
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
