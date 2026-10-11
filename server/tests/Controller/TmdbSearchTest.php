<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Entity\ActivityEntry;
use App\Entity\MetadataCacheEntry;
use App\Entity\User;
use App\Metadata\TmdbSearch;
use App\Security\RequestLimiter;
use App\Tests\Support\DeviceAuthTestTrait;
use App\Tests\Support\FakeTmdbApi;
use App\Tests\Support\RecordingLogger;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\HttpClient\Exception\TransportException;

/**
 * Searching films and series by title in TMDB through the server (7.8, D62). The title a person looks for is the most
 * private thing here: it never travels in an address, is never kept in the clear and never reaches a log. Hostile
 * cases first: no session, a body that is not the one asked, a title that tries to be more than a title, the token
 * showing, a TMDB that sends rubbish or says no, a detail that fails, an account or the day over its limit.
 */
final class TmdbSearchTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string TOKEN = 'SENTINEL-tmdb-token-91ac';
    private const string SECRET_TITLE = 'zxqv private title';

    protected function tearDown(): void
    {
        $_ENV['TMDB_API_TOKEN'] = $_SERVER['TMDB_API_TOKEN'] = '';
        FakeTmdbApi::reset();
        RecordingLogger::reset();
        parent::tearDown();
    }

    // --- The way -----------------------------------------------------------------------------------------------------

    public function testAFilmSearchGivesTheFixedShapeWithItsDetail(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchFinds([FakeTmdbApi::film()]);
        FakeTmdbApi::detailIs('movie', 603, FakeTmdbApi::filmDetail());

        $body = $this->search($client, $ada, 'film', 'matrix', 'es');

        self::assertResponseStatusCodeSame(200);
        self::assertSame(['candidates'], array_keys($body));
        self::assertSame([[
            'external_id' => 603,
            'title' => 'Matrix',
            'original_title' => 'The Matrix',
            'year' => 1999,
            'language' => 'en',
            'cover_url' => 'https://image.tmdb.org/t/p/w500/f89U3ADr1oiB1s9GkdPOEpXUk5H.jpg',
            'directors' => ['Lana Wachowski', 'Lilly Wachowski'],
            'creators' => [],
            'duration_minutes' => 136,
            'seasons' => null,
        ]], $body['candidates']);
        $search = FakeTmdbApi::$requests[0];
        self::assertSame('GET', $search['method']);
        self::assertSame('/3/search/movie', $search['path']);
        self::assertSame('matrix', $search['query']['query'] ?? null);
        self::assertSame('es-ES', $search['query']['language'] ?? null);
        self::assertSame('false', $search['query']['include_adult'] ?? null);
        self::assertSame(['Authorization: Bearer '.self::TOKEN], $search['headers']['authorization'] ?? null);
        self::assertStringNotContainsString(self::TOKEN, $search['url']);
        self::assertSame('/3/movie/603', FakeTmdbApi::$requests[1]['path']);
        self::assertStringNotContainsString(self::TOKEN, FakeTmdbApi::$requests[1]['url']);
    }

    public function testASeriesSearchGivesCreatorsAndSeasons(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchFinds([FakeTmdbApi::show()]);
        FakeTmdbApi::detailIs('tv', 1396, FakeTmdbApi::showDetail());

        $body = $this->search($client, $ada, 'tv_series', 'breaking bad', 'en');

        $candidate = $this->candidates($body)[0];
        self::assertSame('Breaking Bad', $candidate['title']);
        self::assertSame(2008, $candidate['year']);
        self::assertSame(['Vince Gilligan'], $candidate['creators']);
        self::assertSame([], $candidate['directors']);
        self::assertNull($candidate['duration_minutes']);
        self::assertSame(5, $candidate['seasons']);
        self::assertSame('/3/search/tv', FakeTmdbApi::$requests[0]['path']);
        self::assertSame('en-US', FakeTmdbApi::$requests[0]['query']['language'] ?? null);
    }

    public function testNothingFoundIsAnAnswerAndIsRemembered(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchFinds([]);

        $first = $this->search($client, $ada, 'film', 'qwertyuiop', 'en');
        $second = $this->search($client, $ada, 'film', 'qwertyuiop', 'en');

        self::assertResponseStatusCodeSame(200);
        self::assertSame(['candidates' => []], $first);
        self::assertSame($first, $second);
        self::assertSame(1, FakeTmdbApi::calls());
    }

    public function testTheServerListsTmdbOnlyWithAToken(): void
    {
        $client = $this->world();
        $client->request('GET', '/v1/server');
        self::assertContains('tmdb', $this->offered($client));

        $_ENV['TMDB_API_TOKEN'] = $_SERVER['TMDB_API_TOKEN'] = '';
        self::ensureKernelShutdown();
        $client = self::createClient();
        $client->request('GET', '/v1/server');
        self::assertNotContains('tmdb', $this->offered($client));
    }

    public function testWithoutATokenTheRouteIsUnavailableAndNobodyIsAsked(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $_ENV['TMDB_API_TOKEN'] = $_SERVER['TMDB_API_TOKEN'] = '';
        self::ensureKernelShutdown();
        $client = self::createClient();

        $body = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertResponseStatusCodeSame(404);
        self::assertSame('metadata_unavailable', $body['error'] ?? null);
        self::assertSame(0, FakeTmdbApi::calls());
    }

    // --- 1. Who may ask -----------------------------------------------------------------------------------------------

    public function testWithoutASessionThereIsNoAnswerAndNoCall(): void
    {
        $client = $this->world();

        $client->request('POST', '/v1/metadata/tmdb/search', server: ['CONTENT_TYPE' => 'application/json'], content: $this->payload('film', 'matrix', 'en'));

        self::assertResponseStatusCodeSame(401);
        self::assertSame(0, FakeTmdbApi::calls());
    }

    public function testAnInvalidTokenGetsNoAnswerAndNoCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        $this->search($client, ['headers' => $this->deviceHeaders($ada['device_id'], 'nope')], 'film', 'matrix', 'en');

        self::assertResponseStatusCodeSame(401);
        self::assertSame(0, FakeTmdbApi::calls());
    }

    public function testAnAccountNoLongerConfirmedGetsNoAnswerAndNoCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $this->entityManager()->createQuery(\sprintf('UPDATE %s u SET u.confirmedAt = NULL', User::class))->execute();

        $body = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertResponseStatusCodeSame(403);
        self::assertSame('account_not_confirmed', $body['error'] ?? null);
        self::assertSame(0, FakeTmdbApi::calls());
    }

    public function testABlockedAccountGetsNoAnswerAndNoCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $this->user('ada@example.com')->block(new \DateTimeImmutable());
        $this->entityManager()->flush();

        $body = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertResponseStatusCodeSame(403);
        self::assertSame('account_blocked', $body['error'] ?? null);
        self::assertSame(0, FakeTmdbApi::calls());
    }

    // --- 2. A body that is not the one asked -------------------------------------------------------------------------

    /** @return iterable<string, array{string}> */
    public static function badBodies(): iterable
    {
        yield 'a kind that is not on the list' => ['{"kind":"book","query":"matrix","language":"en"}'];
        yield 'no kind' => ['{"query":"matrix","language":"en"}'];
        yield 'a kind that is not text' => ['{"kind":["film"],"query":"matrix","language":"en"}'];
        yield 'an empty query' => ['{"kind":"film","query":"","language":"en"}'];
        yield 'a query of blanks' => ['{"kind":"film","query":"   ","language":"en"}'];
        yield 'a query of one letter' => ['{"kind":"film","query":"a","language":"en"}'];
        yield 'no query' => ['{"kind":"film","language":"en"}'];
        yield 'a query that is not text' => ['{"kind":"film","query":{"a":1},"language":"en"}'];
        yield 'a line break' => ['{"kind":"film","query":"ma\ntrix","language":"en"}'];
        yield 'a tab' => ['{"kind":"film","query":"ma\ttrix","language":"en"}'];
        yield 'a null byte' => ['{"kind":"film","query":"ma\u0000trix","language":"en"}'];
        yield 'a control character' => ['{"kind":"film","query":"ma\u0007trix","language":"en"}'];
        yield 'more fields' => ['{"kind":"film","query":"matrix","language":"en","api_key":"x"}'];
        yield 'broken json' => ['{"kind":"film","query":"matrix"'];
        yield 'a list' => ['["film","matrix"]'];
        yield 'a string' => ['"matrix"'];
        yield 'nothing' => [''];
    }

    #[DataProvider('badBodies')]
    public function testABodyThatIsNotTheOneAskedIsRefusedWithoutTmdbAndWithoutARow(string $body): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        $client->request('POST', '/v1/metadata/tmdb/search', server: $ada['headers'] + ['CONTENT_TYPE' => 'application/json'], content: $body);

        self::assertGreaterThanOrEqual(400, $client->getResponse()->getStatusCode());
        self::assertLessThan(500, $client->getResponse()->getStatusCode());
        self::assertSame(0, FakeTmdbApi::calls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testAQueryOfOneOverTheLimitAndOfAMegabyteAreRefused(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        foreach ([str_repeat('x', 101), str_repeat('x', 1_000_000)] as $query) {
            $this->search($client, $ada, 'film', $query, 'en');
            self::assertResponseStatusCodeSame(422);
        }

        self::assertSame(0, FakeTmdbApi::calls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testAQueryOfHundredCharactersIsEnough(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchFinds([]);

        $this->search($client, $ada, 'film', str_repeat('é', 100), 'en');

        self::assertResponseStatusCodeSame(200);
    }

    public function testAnOddLanguageMeansEnglishAndTheKnownOnesAreKept(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        foreach (['fr' => 'en-US', 'ES' => 'en-US', '' => 'en-US', '../x' => 'en-US', 'es' => 'es-ES', 'en' => 'en-US'] as $given => $sent) {
            FakeTmdbApi::searchFinds([]);
            $this->search($client, $ada, 'film', 'query '.bin2hex((string) $given), (string) $given);
            self::assertResponseStatusCodeSame(200, 'for '.$given);
            $requests = FakeTmdbApi::$requests;
            self::assertSame($sent, $requests[\count($requests) - 1]['query']['language'] ?? null, 'for '.$given);
        }
    }

    public function testSurroundingBlanksAndRepeatedSpacesDoNotChangeWhatIsAsked(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchFinds([]);

        $this->search($client, $ada, 'film', "  The   Matrix \u{a0}", 'en');

        self::assertSame('the matrix', FakeTmdbApi::$requests[0]['query']['query'] ?? null);
    }

    // --- 3. A title that tries to be more than a title ---------------------------------------------------------------

    public function testATitleWithSymbolsTravelsAsOneEncodedParameter(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        foreach (['a&include_adult=true', 'what? #1', 'a/../../b', '100%00 sure', 'x&api_key=1&language=fr', 'ñ ü 映画'] as $title) {
            FakeTmdbApi::searchFinds([]);
            $this->search($client, $ada, 'film', $title, 'en');
            self::assertResponseStatusCodeSame(200, $title);
            $request = FakeTmdbApi::$requests[\count(FakeTmdbApi::$requests) - 1];
            self::assertSame('/3/search/movie', $request['path'], $title);
            self::assertSame(mb_strtolower($title), $request['query']['query'] ?? null, $title);
            self::assertEqualsCanonicalizing(['query', 'include_adult', 'language', 'page'], array_keys($request['query']), $title);
            self::assertSame('false', $request['query']['include_adult']);
            self::assertSame('en-US', $request['query']['language']);
            self::assertStringStartsWith('https://api.themoviedb.org/3/search/movie?', $request['url']);
            self::assertSame(1, substr_count($request['url'], '?'), $title);
            self::assertSame(0, substr_count($request['url'], '#'), $title);
        }
    }

    // --- 4. The token never shows ------------------------------------------------------------------------------------

    public function testTheTokenIsNowhereInWhatTheServerAnswersStoresOrLogs(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $seen = [];
        $n = 0;

        foreach ([401, 429, 500, 404] as $status) {
            $times = 500 === $status ? 3 : 1;
            for ($i = 0; $i < $times; ++$i) {
                FakeTmdbApi::searchAnswers($status, '{"status_message":"bad token '.self::TOKEN.'"}');
            }
            $this->clearBrake();
            $seen[] = $this->raw($client, $ada, 'film', 'query '.++$n);
        }
        FakeTmdbApi::searchFails(new TransportException('Could not resolve https://api.themoviedb.org/3/search/movie with '.self::TOKEN));
        $this->clearBrake();
        $seen[] = $this->raw($client, $ada, 'film', 'query '.++$n);
        FakeTmdbApi::searchFinds([FakeTmdbApi::film()]);
        FakeTmdbApi::detailAnswers('movie', 603, 500, self::TOKEN);
        $this->clearBrake();
        $seen[] = $this->raw($client, $ada, 'film', 'query '.++$n);

        foreach ($seen as $answer) {
            self::assertStringNotContainsString(self::TOKEN, $answer);
        }
        foreach ($this->allCacheRowsAsText() as $row) {
            self::assertStringNotContainsString(self::TOKEN, $row);
        }
        self::assertStringNotContainsString(self::TOKEN, RecordingLogger::everything());
    }

    // --- 5. A TMDB that sends rubbish ---------------------------------------------------------------------------------

    public function testAThousandResultsReachTheAppAsEightAndOnlyEightDetailsAreAsked(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $results = [];
        for ($i = 1; $i <= 1000; ++$i) {
            $results[] = FakeTmdbApi::film($i, 'T'.$i);
        }
        FakeTmdbApi::searchFinds($results);

        $body = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertResponseStatusCodeSame(200);
        self::assertSame(range(1, 8), array_column($this->candidates($body), 'external_id'));
        self::assertSame(8, FakeTmdbApi::detailCalls());
    }

    public function testTheDirectorIsFoundAtTheEndOfALongCrew(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchFinds([FakeTmdbApi::film()]);
        // A big film lists hundreds of people before its director.
        $crew = array_fill(0, 900, ['job' => 'Grip', 'name' => 'Somebody']);
        $crew[] = ['job' => 'Director', 'name' => 'Lana Wachowski'];
        FakeTmdbApi::detailIs('movie', 603, ['runtime' => 136, 'credits' => ['crew' => $crew]]);

        $body = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertResponseStatusCodeSame(200);
        self::assertSame(['Lana Wachowski'], $this->candidates($body)[0]['directors']);
    }

    public function testFieldsOfTheWrongTypeAndHugeOnesAreCutOrDropped(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchFinds([
            ['id' => 7, 'title' => str_repeat('T', 30_000), 'original_title' => ['x'], 'release_date' => 'long ago', 'original_language' => '<script>', 'poster_path' => 42, 'overview' => str_repeat('o', 1000)],
            ['id' => 'abc', 'title' => 'string id'],
            ['id' => [1], 'title' => 'array id'],
            ['id' => -4, 'title' => 'negative id'],
            ['id' => 1.5, 'title' => 'float id'],
            ['id' => 99_999_999_999_999, 'title' => 'too big id'],
            ['id' => 0, 'title' => 'zero id'],
            ['title' => 'no id'],
            'not an object',
        ]);
        FakeTmdbApi::detailIs('movie', 7, ['runtime' => -3, 'credits' => ['crew' => [['job' => 'Director', 'name' => str_repeat('N', 5000)], ['job' => ['x'], 'name' => 'A'], 'x']]]);

        $body = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertResponseStatusCodeSame(200);
        $candidates = $this->candidates($body);
        self::assertSame([7], array_column($candidates, 'external_id'));
        $candidate = $candidates[0];
        self::assertIsString($candidate['title']);
        self::assertSame(300, mb_strlen($candidate['title']));
        self::assertNull($candidate['original_title']);
        self::assertNull($candidate['year']);
        self::assertNull($candidate['language']);
        self::assertNull($candidate['cover_url']);
        self::assertNull($candidate['duration_minutes']);
        self::assertSame([str_repeat('N', 200)], $candidate['directors']);
        // Only the id TMDB itself gave, as a number, is ever put in an address.
        self::assertSame(['/3/search/movie', '/3/movie/7'], array_map(static fn (array $r): string => $r['path'], FakeTmdbApi::$requests));
    }

    /** @return iterable<string, array{mixed}> */
    public static function posters(): iterable
    {
        yield 'a parent directory' => ['/../x.jpg'];
        yield 'another site' => ['https://evil.example/x.jpg'];
        yield 'a protocol-relative one' => ['//evil.example/x.jpg'];
        yield 'a subdirectory' => ['/a/b.jpg'];
        yield 'another extension' => ['/x.svg'];
        yield 'no extension' => ['/x'];
        yield 'no slash' => ['x.jpg'];
        yield 'a query' => ['/x.jpg?evil=1'];
        yield 'a line break' => ["/x.jpg\n"];
        yield 'dots' => ['/..jpg'];
        yield 'very long' => ['/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.jpg'];
        yield 'an array' => [['/x.jpg']];
    }

    #[DataProvider('posters')]
    public function testAPosterThatIsNotAPlainFileIsDropped(mixed $poster): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchFinds([['id' => 5, 'title' => 'T', 'poster_path' => $poster]]);

        $body = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertResponseStatusCodeSame(200);
        self::assertNull($this->candidates($body)[0]['cover_url']);
    }

    public function testAPlainPosterBecomesAnAddressOnTheirImageHost(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchFinds([['id' => 5, 'title' => 'T', 'poster_path' => '/AbC123.png']]);

        $body = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertSame('https://image.tmdb.org/t/p/w500/AbC123.png', $this->candidates($body)[0]['cover_url']);
    }

    public function testAnswersThatAreNotUsableAreAFailureAndLeaveNoRow(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $n = 0;

        foreach (['<html>hello</html>', '[1,2,3', 'null', '"text"', '{"results":"no"}', '{}', str_repeat('{"a":1,', 400_000)] as $nonsense) {
            FakeTmdbApi::searchAnswers(200, $nonsense);
            $body = $this->search($client, $ada, 'film', 'query '.++$n, 'en');
            self::assertResponseStatusCodeSame(502, substr($nonsense, 0, 20));
            self::assertSame('metadata_failed', $body['error'] ?? null);
        }

        self::assertSame(0, $this->cacheRows());
    }

    public function testAnEnormousAnswerIsAFailureAndLeavesNoRow(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchFinds([FakeTmdbApi::film(1, str_repeat('T', 1_000_000))]);

        $body = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertResponseStatusCodeSame(502);
        self::assertSame('metadata_failed', $body['error'] ?? null);
        self::assertSame(0, $this->cacheRows());
    }

    // --- 6. TMDB says no ----------------------------------------------------------------------------------------------

    /** @return iterable<string, array{int}> */
    public static function refusals(): iterable
    {
        yield 'too many requests' => [429];
        yield 'unauthorised' => [401];
    }

    #[DataProvider('refusals')]
    public function testTmdbRefusingIsReportedAndNothingIsStored(int $status): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchAnswers($status, '{"status_code":7}');

        $body = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertResponseStatusCodeSame(503);
        self::assertSame('metadata_quota_exhausted', $body['error'] ?? null);
        self::assertSame(0, $this->cacheRows());
    }

    public function testAfterARefusalTheServerStopsAskingTmdbForAWhile(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchAnswers(429, '{}');

        $this->search($client, $ada, 'film', 'matrix', 'en');
        for ($try = 0; $try < 5; ++$try) {
            $body = $this->search($client, $ada, 'tv_series', 'other '.$try, 'en');
            self::assertSame('metadata_quota_exhausted', $body['error'] ?? null);
        }

        self::assertSame(1, FakeTmdbApi::calls());
        self::assertGreaterThan(0, (int) $client->getResponse()->headers->get('Retry-After'));

        $this->clearBrake();
        FakeTmdbApi::searchFinds([]);
        $this->search($client, $ada, 'film', 'matrix', 'en');
        self::assertResponseStatusCodeSame(200);
        self::assertSame(2, FakeTmdbApi::calls());
    }

    public function testAServerErrorIsTriedAgainAndThenAnswers(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchAnswers(503, 'down');
        FakeTmdbApi::searchFinds([FakeTmdbApi::film()]);
        FakeTmdbApi::detailIs('movie', 603, FakeTmdbApi::filmDetail());

        $body = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertResponseStatusCodeSame(200);
        self::assertCount(1, $this->candidates($body));
        self::assertSame(2, FakeTmdbApi::searchCalls());
    }

    public function testAServerErrorThatKeepsComingStopsAtThreeTriesAndLeavesNoRow(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        for ($i = 0; $i < 5; ++$i) {
            FakeTmdbApi::searchAnswers(500, 'oops');
        }

        $body = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertResponseStatusCodeSame(502);
        self::assertSame('metadata_failed', $body['error'] ?? null);
        self::assertSame(3, FakeTmdbApi::searchCalls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testASlowTmdbIsNotAskedAgain(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchFails(new TransportException('Idle timeout reached'));

        $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertResponseStatusCodeSame(502);
        self::assertSame(1, FakeTmdbApi::calls());
    }

    // --- 7. A detail that fails ---------------------------------------------------------------------------------------

    public function testADetailThatFailsLeavesThatCandidateIncompleteAndTheSearchAnswers(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchFinds([FakeTmdbApi::film(1, 'One'), FakeTmdbApi::film(2, 'Two'), FakeTmdbApi::film(3, 'Three'), FakeTmdbApi::film(4, 'Four'), FakeTmdbApi::film(5, 'Five')]);
        FakeTmdbApi::detailIs('movie', 1, FakeTmdbApi::filmDetail());
        FakeTmdbApi::detailAnswers('movie', 2, 500, 'oops');
        FakeTmdbApi::detailFails('movie', 3, new TransportException('Idle timeout reached'));
        FakeTmdbApi::detailAnswers('movie', 4, 200, '<html>');
        FakeTmdbApi::detailAnswers('movie', 5, 429, '{}');

        $body = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertResponseStatusCodeSame(200);
        $candidates = $this->candidates($body);
        self::assertCount(5, $candidates);
        self::assertSame(136, $candidates[0]['duration_minutes']);
        foreach (\array_slice($candidates, 1) as $candidate) {
            self::assertNull($candidate['duration_minutes']);
            self::assertSame([], $candidate['directors']);
            self::assertNotNull($candidate['title']);
        }
    }

    public function testADetailRefusedOnQuotaPausesTmdbForTheNextSearch(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchFinds([FakeTmdbApi::film()]);
        FakeTmdbApi::detailAnswers('movie', 603, 429, '{}');

        $this->search($client, $ada, 'film', 'matrix', 'en');
        self::assertResponseStatusCodeSame(200);
        $calls = FakeTmdbApi::calls();
        $body = $this->search($client, $ada, 'film', 'something else', 'en');

        self::assertResponseStatusCodeSame(503);
        self::assertSame('metadata_quota_exhausted', $body['error'] ?? null);
        self::assertSame($calls, FakeTmdbApi::calls());
    }

    public function testADetailThatFailedIsNotRememberedAndIsAskedAgainNextTime(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchFinds([FakeTmdbApi::film()]);
        FakeTmdbApi::detailAnswers('movie', 603, 500, 'oops');

        $this->search($client, $ada, 'film', 'matrix', 'en');
        self::assertSame(0, $this->detailRows());
        FakeTmdbApi::detailIs('movie', 603, FakeTmdbApi::filmDetail());
        $second = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertSame(136, $this->candidates($second)[0]['duration_minutes']);
        self::assertSame(1, FakeTmdbApi::searchCalls());
        self::assertSame(1, $this->detailRows());
    }

    // --- 8. Nobody is remembered --------------------------------------------------------------------------------------

    public function testTwoAccountsLookingForTheSameCostOneRoundOfCallsAndNoRowSaysWhoOrWhat(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        FakeTmdbApi::searchFinds([FakeTmdbApi::film()]);
        FakeTmdbApi::detailIs('movie', 603, FakeTmdbApi::filmDetail());
        $activity = $this->entityManager()->getRepository(ActivityEntry::class)->count([]);

        $this->search($client, $ada, 'film', self::SECRET_TITLE, 'en');
        $this->search($client, $grace, 'film', '  '.mb_strtoupper(self::SECRET_TITLE).' ', 'en');

        self::assertSame(2, FakeTmdbApi::calls());
        self::assertSame(2, $this->cacheRows());
        self::assertSame(['id', 'provider', 'code', 'found', 'payload', 'fetched_at'], $this->cacheColumns());
        $rows = implode(' ', $this->allCacheRowsAsText());
        self::assertStringNotContainsString('ada@example.com', $rows);
        self::assertStringNotContainsString('grace@example.com', $rows);
        self::assertStringNotContainsString($this->user('ada@example.com')->getId()->toRfc4122(), $rows);
        self::assertStringNotContainsStringIgnoringCase('zxqv', $rows);
        self::assertStringNotContainsStringIgnoringCase('private title', $rows);
        self::assertContains(TmdbSearch::fingerprint('film', 'en', self::SECRET_TITLE), $this->codes());
        self::assertSame($activity, $this->entityManager()->getRepository(ActivityEntry::class)->count([]));
    }

    public function testTheSearchedTitleReachesNoLogEvenWhenSomethingGoesWrong(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchAnswers(429, '{}');
        $this->search($client, $ada, 'film', self::SECRET_TITLE, 'en');
        $this->clearBrake();
        FakeTmdbApi::searchAnswers(500, 'x');
        FakeTmdbApi::searchAnswers(500, 'x');
        FakeTmdbApi::searchAnswers(500, 'x');
        $this->search($client, $ada, 'film', self::SECRET_TITLE.' two', 'en');
        $this->search($client, $ada, 'book', self::SECRET_TITLE, 'en');
        $this->search($client, $ada, 'film', self::SECRET_TITLE."\n", 'en');
        $client->request('POST', '/v1/metadata/tmdb/search', server: $ada['headers'] + ['CONTENT_TYPE' => 'application/json'], content: '{"kind":"film","query":"'.self::SECRET_TITLE.'"');
        $client->request('POST', '/v1/metadata/tmdb/search', server: $ada['headers'] + ['CONTENT_TYPE' => 'application/json'], content: '{"kind":"film","query":"'.self::SECRET_TITLE.'","extra":1}');

        self::assertNotSame('', RecordingLogger::everything(), 'the logger must be the one the test reads');
        self::assertStringNotContainsStringIgnoringCase('zxqv', RecordingLogger::everything());
        self::assertStringNotContainsStringIgnoringCase('private title', RecordingLogger::everything());
    }

    // --- 9. Limits ----------------------------------------------------------------------------------------------------

    public function testAnAccountPastItsSharedLimitIsCutAndTheOthersAreNot(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        $limiter = self::getContainer()->get(RequestLimiter::class);
        // The limit is the same as Google Books': what an account spends there is spent here.
        for ($i = 0; $i < 599; ++$i) {
            $limiter->consumeMetadata($this->user('ada@example.com')->getId()->toRfc4122());
        }
        FakeTmdbApi::searchFinds([]);
        FakeTmdbApi::searchFinds([]);

        $this->search($client, $ada, 'film', 'matrix', 'en');
        self::assertResponseStatusCodeSame(200);
        $body = $this->search($client, $ada, 'film', 'matrix', 'en');
        self::assertResponseStatusCodeSame(429);
        self::assertSame('too_many_requests', $body['error'] ?? null);

        $this->search($client, $grace, 'film', 'matrix', 'en');
        self::assertResponseStatusCodeSame(200);
    }

    public function testWhenTheServersDayIsSpentTheCacheKeepsServing(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeTmdbApi::searchFinds([FakeTmdbApi::film()]);
        FakeTmdbApi::detailIs('movie', 603, FakeTmdbApi::filmDetail());
        $this->search($client, $ada, 'film', 'matrix', 'en');
        $limiter = self::getContainer()->get(RequestLimiter::class);
        for ($i = 0; $i < 4998; ++$i) {
            $limiter->consumeTmdbCall(); // with the two calls made above, the 5,000 of the day
        }

        $cached = $this->search($client, $ada, 'film', 'matrix', 'en');
        self::assertResponseStatusCodeSame(200);
        self::assertSame('Matrix', $this->candidates($cached)[0]['title']);
        self::assertSame(136, $this->candidates($cached)[0]['duration_minutes']);

        $other = $this->search($client, $ada, 'film', 'other', 'en');
        self::assertResponseStatusCodeSame(503);
        self::assertSame('metadata_quota_exhausted', $other['error'] ?? null);
        self::assertSame(2, FakeTmdbApi::calls());
    }

    public function testTheDayRunningOutWhileTheDetailsAreAskedLeavesTheRestIncomplete(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $limiter = self::getContainer()->get(RequestLimiter::class);
        for ($i = 0; $i < 4_998; ++$i) {
            $limiter->consumeTmdbCall(); // the search and one detail are what is left
        }
        FakeTmdbApi::searchFinds([FakeTmdbApi::film(1, 'One'), FakeTmdbApi::film(2, 'Two'), FakeTmdbApi::film(3, 'Three')]);
        FakeTmdbApi::detailIs('movie', 1, FakeTmdbApi::filmDetail());
        FakeTmdbApi::detailIs('movie', 2, FakeTmdbApi::filmDetail());
        FakeTmdbApi::detailIs('movie', 3, FakeTmdbApi::filmDetail());

        $body = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertResponseStatusCodeSame(200);
        self::assertCount(3, $this->candidates($body));
        self::assertSame(2, FakeTmdbApi::calls());
    }

    // --- The cache's age ------------------------------------------------------------------------------------------------

    public function testASearchIsKeptADayAndADetailThirtyDays(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $now = new \DateTimeImmutable();
        $this->store('tmdb_search', TmdbSearch::fingerprint('film', 'en', 'matrix'), true, ['hits' => [['external_id' => 603, 'title' => 'Matrix', 'original_title' => null, 'year' => 1999, 'language' => 'en', 'cover_url' => null]]], $now->modify('-23 hours'));
        $this->store('tmdb_movie_en', '603', true, ['directors' => ['Cached Director'], 'creators' => [], 'duration_minutes' => 100, 'seasons' => null], $now->modify('-29 days'));

        $body = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertSame(0, FakeTmdbApi::calls());
        self::assertSame(['Cached Director'], $this->candidates($body)[0]['directors']);

        $this->entityManager()->createQuery(\sprintf('UPDATE %s e SET e.fetchedAt = :at WHERE e.provider = :provider', MetadataCacheEntry::class))
            ->setParameter('at', $now->modify('-25 hours'))->setParameter('provider', 'tmdb_search')->execute();
        $this->entityManager()->createQuery(\sprintf('UPDATE %s e SET e.fetchedAt = :at WHERE e.provider = :provider', MetadataCacheEntry::class))
            ->setParameter('at', $now->modify('-31 days'))->setParameter('provider', 'tmdb_movie_en')->execute();
        $this->entityManager()->clear();
        FakeTmdbApi::searchFinds([FakeTmdbApi::film()]);
        FakeTmdbApi::detailIs('movie', 603, FakeTmdbApi::filmDetail());

        $again = $this->search($client, $ada, 'film', 'matrix', 'en');

        self::assertSame(2, FakeTmdbApi::calls());
        self::assertSame(['Lana Wachowski', 'Lilly Wachowski'], $this->candidates($again)[0]['directors']);
    }

    // --- helpers ------------------------------------------------------------------------------------------------------

    private function world(): KernelBrowser
    {
        $_ENV['TMDB_API_TOKEN'] = $_SERVER['TMDB_API_TOKEN'] = self::TOKEN;
        self::ensureKernelShutdown();
        $client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();
        $this->clearBrake();
        FakeTmdbApi::reset();
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

    private function payload(string $kind, string $query, string $language): string
    {
        return json_encode(['kind' => $kind, 'query' => $query, 'language' => $language], \JSON_THROW_ON_ERROR | \JSON_INVALID_UTF8_SUBSTITUTE);
    }

    /**
     * @param array{headers: array<string, string>} $session
     *
     * @return array<string, mixed>
     */
    private function search(KernelBrowser $client, array $session, string $kind, string $query, string $language): array
    {
        $this->raw($client, $session, $kind, $query, $language);

        return $this->decode($client);
    }

    /** @param array{headers: array<string, string>} $session */
    private function raw(KernelBrowser $client, array $session, string $kind, string $query, string $language = 'en'): string
    {
        $client->request('POST', '/v1/metadata/tmdb/search', server: $session['headers'] + ['CONTENT_TYPE' => 'application/json'], content: $this->payload($kind, $query, $language));

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
        return $this->numberFrom("SELECT COUNT(*) FROM metadata_cache WHERE provider LIKE 'tmdb\\_movie%' OR provider LIKE 'tmdb\\_tv%'");
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
