<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Entity\MetadataCacheEntry;
use App\Entity\User;
use App\Metadata\GameUpcUserId;
use App\Tests\Support\DeviceAuthTestTrait;
use App\Tests\Support\FakeGameUpcApi;
use App\Tests\Support\RecordingLogger;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\HttpClient\Exception\TransportException;

/**
 * Which board game a barcode is, through GameUPC, and the game a person chose sent back to it (D62). Delicate in
 * three ways: the key (nowhere but the header of the outgoing call), what comes back (from outside, checked field by
 * field) and the choices, which change GameUPC's data under our key and carry an id per account that must say nothing
 * about who that is. Hostile cases first.
 */
final class GameUpcTest extends WebTestCase
{
    use DeviceAuthTestTrait;

    private const string KEY = 'SENTINEL-gameupc-key-7c3a';
    private const string GLOOMHAVEN = '019962194719';
    private const string TINY_TOWNS = '222222222224';
    private const string UNKNOWN = '333333333331';

    protected function tearDown(): void
    {
        $_ENV['GAMEUPC_API_KEY'] = $_SERVER['GAMEUPC_API_KEY'] = '';
        FakeGameUpcApi::reset();
        RecordingLogger::reset();
        parent::tearDown();
    }

    // --- The way -----------------------------------------------------------------------------------------------------

    public function testAVerifiedBarcodeGivesOneSureGame(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGameUpcApi::lookAnswers(FakeGameUpcApi::verified());

        $body = $this->lookup($client, $ada, self::GLOOMHAVEN);

        self::assertResponseStatusCodeSame(200);
        self::assertSame([
            'status' => 'verified',
            'candidates' => [['external_id' => 174430, 'title' => 'Gloomhaven', 'cover_url' => FakeGameUpcApi::GLOOMHAVEN_COVER]],
        ], $body);
        self::assertStringContainsString('no-store', (string) $client->getResponse()->headers->get('Cache-Control'));
        $request = FakeGameUpcApi::$requests[0];
        self::assertSame('GET', $request['method']);
        self::assertSame('https://api.gameupc.com/v1/upc/'.self::GLOOMHAVEN, $request['url']);
        self::assertSame(['x-api-key: '.self::KEY], $request['headers']['x-api-key'] ?? null);
    }

    public function testGuessesAreForThePersonToChooseFrom(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGameUpcApi::lookAnswers(FakeGameUpcApi::guesses());

        $body = $this->lookup($client, $ada, self::TINY_TOWNS);

        self::assertResponseStatusCodeSame(200);
        self::assertSame([
            'status' => 'choose',
            'candidates' => [
                ['external_id' => 265736, 'title' => 'Tiny Towns', 'cover_url' => null],
                ['external_id' => 287576, 'title' => 'Tiny Towns: Fortune', 'cover_url' => null],
            ],
        ], $body);
    }

    public function testABarcodeItKnowsNothingOfIsAnAnswer(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGameUpcApi::lookAnswers(FakeGameUpcApi::nothing());

        $body = $this->lookup($client, $ada, self::UNKNOWN);

        self::assertResponseStatusCodeSame(200);
        self::assertSame(['status' => 'unknown', 'candidates' => []], $body);
    }

    public function testAVerifiedStatusWithoutAGameIsUnknown(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGameUpcApi::lookAnswers(['bgg_info_status' => 'verified', 'bgg_info' => [FakeGameUpcApi::game('x', 'Not a game')]]);

        self::assertSame(['status' => 'unknown', 'candidates' => []], $this->lookup($client, $ada, self::GLOOMHAVEN));
    }

    public function testAChoiceIsSentWithTheAccountsGameUpcIdAndNothingElse(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGameUpcApi::choiceAnswers();

        $this->choose($client, $ada, self::TINY_TOWNS, 265736);

        self::assertResponseStatusCodeSame(204);
        $request = FakeGameUpcApi::$requests[0];
        self::assertSame('POST', $request['method']);
        self::assertSame('https://api.gameupc.com/v1/upc/'.self::TINY_TOWNS.'/bgg_id/265736', $request['url']);
        self::assertSame(['x-api-key: '.self::KEY], $request['headers']['x-api-key'] ?? null);
        $sent = json_decode($request['body'], true);
        self::assertIsArray($sent);
        self::assertSame(['user_id'], array_keys($sent));
        self::assertSame($this->gameUpcIdOf('ada@example.com'), $sent['user_id']);
    }

    public function testTheServerListsGameUpcOnlyWithAKey(): void
    {
        $client = $this->world();
        $client->request('GET', '/v1/server');
        self::assertContains('gameupc', $this->offered($client));

        $_ENV['GAMEUPC_API_KEY'] = $_SERVER['GAMEUPC_API_KEY'] = '';
        self::ensureKernelShutdown();
        $client = self::createClient();
        $client->request('GET', '/v1/server');
        self::assertNotContains('gameupc', $this->offered($client));
    }

    public function testWithoutAKeyBothRoutesAreUnavailableAndNobodyIsAsked(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $_ENV['GAMEUPC_API_KEY'] = $_SERVER['GAMEUPC_API_KEY'] = '';
        self::ensureKernelShutdown();
        $client = self::createClient();

        $looked = $this->lookup($client, $ada, self::GLOOMHAVEN);
        self::assertResponseStatusCodeSame(404);
        self::assertSame('metadata_unavailable', $looked['error'] ?? null);
        $chosen = $this->choose($client, $ada, self::GLOOMHAVEN, 174430);
        self::assertResponseStatusCodeSame(404);
        self::assertSame('metadata_unavailable', $chosen['error'] ?? null);
        self::assertSame(0, FakeGameUpcApi::calls());
    }

    // --- 1. Who may ask -----------------------------------------------------------------------------------------------

    public function testWithoutASessionThereIsNoAnswerAndNoCall(): void
    {
        $client = $this->world();

        $client->request('POST', '/v1/metadata/gameupc/lookup', server: ['CONTENT_TYPE' => 'application/json'], content: '{"code":"'.self::GLOOMHAVEN.'"}');
        self::assertResponseStatusCodeSame(401);
        $client->request('POST', '/v1/metadata/gameupc/choice', server: ['CONTENT_TYPE' => 'application/json'], content: '{"code":"'.self::GLOOMHAVEN.'","external_id":174430}');
        self::assertResponseStatusCodeSame(401);

        self::assertSame(0, FakeGameUpcApi::calls());
    }

    public function testABlockedAccountGetsNoAnswerAndSendsNoChoice(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $this->user('ada@example.com')->block(new \DateTimeImmutable());
        $this->entityManager()->flush();

        $looked = $this->lookup($client, $ada, self::GLOOMHAVEN);
        self::assertResponseStatusCodeSame(403);
        self::assertSame('account_blocked', $looked['error'] ?? null);
        $this->choose($client, $ada, self::GLOOMHAVEN, 174430);
        self::assertResponseStatusCodeSame(403);

        self::assertSame(0, FakeGameUpcApi::calls());
    }

    // --- 2. A body that is not the one asked -------------------------------------------------------------------------

    /** @return iterable<string, array{string}> */
    public static function badLookups(): iterable
    {
        yield 'a wrong check digit' => ['{"code":"019962194718"}'];
        yield 'eleven digits' => ['{"code":"01996219471"}'];
        yield 'fifteen digits' => ['{"code":"019962194719000"}'];
        yield 'letters' => ['{"code":"01996219471a"}'];
        yield 'a path' => ['{"code":"../bgg_id/1"}'];
        yield 'another address' => ['{"code":"https://evil.example/"}'];
        yield 'blanks around' => ['{"code":" 019962194719 "}'];
        yield 'a query of its own' => ['{"code":"019962194719?search=x"}'];
        yield 'a number, not text' => ['{"code":19962194719}'];
        yield 'empty' => ['{"code":""}'];
        yield 'no code' => ['{}'];
        yield 'more fields' => ['{"code":"019962194719","search":"gloomhaven"}'];
        yield 'broken json' => ['{"code":"019962194719"'];
        yield 'nothing' => [''];
    }

    #[DataProvider('badLookups')]
    public function testALookupThatIsNotABarcodeIsRefusedWithoutACallOrARow(string $body): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        $client->request('POST', '/v1/metadata/gameupc/lookup', server: $ada['headers'] + ['CONTENT_TYPE' => 'application/json'], content: $body);

        self::assertGreaterThanOrEqual(400, $client->getResponse()->getStatusCode());
        self::assertLessThan(500, $client->getResponse()->getStatusCode());
        self::assertSame(0, FakeGameUpcApi::calls());
        self::assertSame(0, $this->cacheRows());
    }

    /** @return iterable<string, array{string}> */
    public static function badChoices(): iterable
    {
        yield 'a wrong check digit' => ['{"code":"222222222225","external_id":265736}'];
        yield 'a path in the code' => ['{"code":"../../upc/1","external_id":265736}'];
        yield 'no game' => ['{"code":"222222222224"}'];
        yield 'game zero' => ['{"code":"222222222224","external_id":0}'];
        yield 'a negative game' => ['{"code":"222222222224","external_id":-1}'];
        yield 'a game too big' => ['{"code":"222222222224","external_id":10000000000}'];
        yield 'a game as text' => ['{"code":"222222222224","external_id":"265736"}'];
        yield 'a game with a path' => ['{"code":"222222222224","external_id":"1/../2"}'];
        yield 'a user id of its own' => ['{"code":"222222222224","external_id":265736,"user_id":"me"}'];
        yield 'nothing' => [''];
    }

    #[DataProvider('badChoices')]
    public function testAChoiceThatIsNotOneIsRefusedWithoutACall(string $body): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        $client->request('POST', '/v1/metadata/gameupc/choice', server: $ada['headers'] + ['CONTENT_TYPE' => 'application/json'], content: $body);

        self::assertGreaterThanOrEqual(400, $client->getResponse()->getStatusCode());
        self::assertLessThan(500, $client->getResponse()->getStatusCode());
        self::assertSame(0, FakeGameUpcApi::calls());
    }

    public function testTheShortCodesOfABoxPass(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        foreach (['96385074', '4006381333931', '10012345678902'] as $code) {
            FakeGameUpcApi::lookAnswers(FakeGameUpcApi::nothing($code));
            $this->lookup($client, $ada, $code);
            self::assertResponseStatusCodeSame(200, $code);
        }
        self::assertSame(3, FakeGameUpcApi::calls());
    }

    // --- 3. What comes back is from outside ---------------------------------------------------------------------------

    public function testGamesThatAreNotWellFormedAreDroppedAndCoversOffTheirHostToo(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGameUpcApi::lookAnswers(FakeGameUpcApi::guesses([
            FakeGameUpcApi::game('265736', 'An id as text'),
            FakeGameUpcApi::game(0, 'Id zero'),
            FakeGameUpcApi::game(10_000_000_000, 'Id too big'),
            FakeGameUpcApi::game(1, ''),
            FakeGameUpcApi::game(2, ['not', 'text']),
            FakeGameUpcApi::game(3, str_repeat('x', 400)),
            FakeGameUpcApi::game(4, 'Cover elsewhere', 'https://evil.example/pic.jpg'),
            FakeGameUpcApi::game(5, 'Cover with a path', 'https://cf.geekdo-images.com/../evil/pic.jpg'),
            FakeGameUpcApi::game(6, 'Cover over http', 'http://cf.geekdo-images.com/a/pic.jpg'),
            FakeGameUpcApi::game(7, 'Good', FakeGameUpcApi::GLOOMHAVEN_COVER),
        ]));

        $body = $this->lookup($client, $ada, self::TINY_TOWNS);

        self::assertSame('choose', $body['status'] ?? null);
        self::assertSame([
            ['external_id' => 4, 'title' => 'Cover elsewhere', 'cover_url' => null],
            ['external_id' => 5, 'title' => 'Cover with a path', 'cover_url' => null],
            ['external_id' => 6, 'title' => 'Cover over http', 'cover_url' => null],
            ['external_id' => 7, 'title' => 'Good', 'cover_url' => FakeGameUpcApi::GLOOMHAVEN_COVER],
        ], $body['candidates'] ?? null);
    }

    public function testAHundredGuessesReachTheAppAsEight(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGameUpcApi::lookAnswers(FakeGameUpcApi::guesses(array_map(static fn (int $i): array => FakeGameUpcApi::game($i, 'Game '.$i), range(1, 100))));

        $body = $this->lookup($client, $ada, self::TINY_TOWNS);

        self::assertCount(8, \is_array($body['candidates'] ?? null) ? $body['candidates'] : []);
    }

    /** @return iterable<string, array{string}> */
    public static function unusableBodies(): iterable
    {
        yield 'not json' => ['<html>oops</html>'];
        yield 'a string' => ['"verified"'];
        yield 'too deep' => [str_repeat('[', 40).str_repeat(']', 40)];
        yield 'enormous' => ['{"bgg_info":[],"padding":"'.str_repeat('x', 300 * 1024).'"}'];
    }

    #[DataProvider('unusableBodies')]
    public function testAnAnswerThatIsNoUseIsAFailureAndLeavesNoRow(string $text): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGameUpcApi::lookAnswersText($text);

        $body = $this->lookup($client, $ada, self::GLOOMHAVEN);

        self::assertResponseStatusCodeSame(502);
        self::assertSame('metadata_failed', $body['error'] ?? null);
        self::assertSame(0, $this->cacheRows());
    }

    // --- 4. The key ---------------------------------------------------------------------------------------------------

    public function testTheKeyIsNowhereInWhatTheServerAnswersStoresOrLogs(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $seen = [];
        foreach ([401, 429, 400, 500] as $status) {
            $times = 500 === $status ? 3 : 1;
            for ($i = 0; $i < $times; ++$i) {
                FakeGameUpcApi::lookAnswersText('{"status":"error","message":"bad key '.self::KEY.'"}', $status);
            }
            $this->clearBrake();
            $seen[] = $this->raw($client, $ada, self::GLOOMHAVEN);
        }
        FakeGameUpcApi::lookFails(new TransportException('Could not reach api.gameupc.com with x-api-key: '.self::KEY));
        $this->clearBrake();
        $seen[] = $this->raw($client, $ada, self::GLOOMHAVEN);
        FakeGameUpcApi::lookAnswers(FakeGameUpcApi::verified() + ['note' => self::KEY]);
        $this->clearBrake();
        $seen[] = $this->raw($client, $ada, self::GLOOMHAVEN);

        foreach ($seen as $answer) {
            self::assertStringNotContainsString(self::KEY, $answer);
        }
        foreach ($this->allCacheRowsAsText() as $row) {
            self::assertStringNotContainsString(self::KEY, $row);
        }
        self::assertStringNotContainsString(self::KEY, RecordingLogger::everything());
        foreach (FakeGameUpcApi::$requests as $request) {
            self::assertStringNotContainsString(self::KEY, $request['url']);
        }
    }

    // --- 5. The cache -------------------------------------------------------------------------------------------------

    public function testAVerifiedBarcodeIsKeptThirtyDaysAndCostsOneCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        FakeGameUpcApi::lookAnswers(FakeGameUpcApi::verified());

        $first = $this->lookup($client, $ada, self::GLOOMHAVEN);
        $second = $this->lookup($client, $grace, self::GLOOMHAVEN);

        self::assertSame($first, $second);
        self::assertSame(1, FakeGameUpcApi::calls());
        $entry = $this->entry(self::GLOOMHAVEN);
        self::assertTrue($entry->isFreshAt(new \DateTimeImmutable('+29 days')));
        self::assertFalse($entry->isFreshAt(new \DateTimeImmutable('+31 days')));
    }

    public function testGuessesAndNothingAreKeptOneDay(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGameUpcApi::lookAnswers(FakeGameUpcApi::guesses());
        FakeGameUpcApi::lookAnswers(FakeGameUpcApi::nothing());

        $this->lookup($client, $ada, self::TINY_TOWNS);
        $this->lookup($client, $ada, self::UNKNOWN);

        foreach ([self::TINY_TOWNS, self::UNKNOWN] as $code) {
            $entry = $this->entry($code);
            self::assertTrue($entry->isFreshAt(new \DateTimeImmutable('+23 hours')));
            self::assertFalse($entry->isFreshAt(new \DateTimeImmutable('+25 hours')));
        }
    }

    public function testNoRowSaysWhoAsked(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGameUpcApi::lookAnswers(FakeGameUpcApi::guesses());

        $this->lookup($client, $ada, self::TINY_TOWNS);

        self::assertSame(['id', 'provider', 'code', 'found', 'payload', 'fetched_at'], $this->cacheColumns());
        $id = $this->user('ada@example.com')->getId()->toRfc4122();
        foreach ($this->allCacheRowsAsText() as $row) {
            self::assertStringNotContainsString('ada@example.com', $row);
            self::assertStringNotContainsString($id, $row);
            self::assertStringNotContainsString($this->gameUpcIdOf('ada@example.com'), $row);
        }
    }

    public function testACoverInTheCacheThatGameUpcCouldNotHaveSentIsDropped(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $this->store(self::GLOOMHAVEN, true, ['status' => 'verified', 'candidates' => [['external_id' => 174430, 'title' => 'Gloomhaven', 'cover_url' => 'https://evil.example/pic.jpg']]]);

        $body = $this->lookup($client, $ada, self::GLOOMHAVEN);

        self::assertSame([['external_id' => 174430, 'title' => 'Gloomhaven', 'cover_url' => null]], $body['candidates'] ?? null);
        self::assertSame(0, FakeGameUpcApi::calls());
    }

    public function testAChoiceForgetsWhatWasKeptOfThatBarcodeAndOnlyThatOne(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGameUpcApi::lookAnswers(FakeGameUpcApi::guesses());
        FakeGameUpcApi::lookAnswers(FakeGameUpcApi::nothing());
        $this->lookup($client, $ada, self::TINY_TOWNS);
        $this->lookup($client, $ada, self::UNKNOWN);
        FakeGameUpcApi::choiceAnswers();

        $this->choose($client, $ada, self::TINY_TOWNS, 265736);

        self::assertSame([self::UNKNOWN], $this->codes());
    }

    public function testAChoiceThatFailsKeepsWhatWasKept(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGameUpcApi::lookAnswers(FakeGameUpcApi::guesses());
        $this->lookup($client, $ada, self::TINY_TOWNS);
        FakeGameUpcApi::choiceAnswers(400);

        $body = $this->choose($client, $ada, self::TINY_TOWNS, 265736);

        self::assertResponseStatusCodeSame(502);
        self::assertSame('metadata_failed', $body['error'] ?? null);
        self::assertSame([self::TINY_TOWNS], $this->codes());
    }

    // --- 6. The id GameUPC is sent ------------------------------------------------------------------------------------

    public function testTheGameUpcIdIsStablePerAccountDiffersBetweenAccountsAndSaysNothingOfThem(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        FakeGameUpcApi::choiceAnswers();
        FakeGameUpcApi::choiceAnswers();
        FakeGameUpcApi::choiceAnswers();

        $this->choose($client, $ada, self::TINY_TOWNS, 265736);
        $this->choose($client, $ada, self::UNKNOWN, 1);
        $this->choose($client, $grace, self::TINY_TOWNS, 265736);

        $ids = array_map(static function (array $r): mixed {
            $sent = json_decode($r['body'], true);

            return \is_array($sent) ? ($sent['user_id'] ?? null) : null;
        }, FakeGameUpcApi::$requests);
        self::assertSame($ids[0], $ids[1]);
        self::assertNotSame($ids[0], $ids[2]);
        foreach (['ada@example.com', 'grace@example.com'] as $email) {
            $id = $this->user($email)->getId()->toRfc4122();
            foreach (FakeGameUpcApi::$requests as $request) {
                self::assertStringNotContainsString($id, $request['body']);
                self::assertStringNotContainsString(str_replace('-', '', $id), $request['body']);
                self::assertStringNotContainsString($email, $request['body']);
            }
        }
        self::assertMatchesRegularExpression('/^[0-9a-f]{32}$/', \is_string($ids[0]) ? $ids[0] : '');
    }

    public function testTheGameUpcIdDependsOnTheServersSecret(): void
    {
        self::assertNotSame(new GameUpcUserId('one secret')->of('an account'), new GameUpcUserId('another secret')->of('an account'));
        self::assertSame(new GameUpcUserId('one secret')->of('an account'), new GameUpcUserId('one secret')->of('an account'));
    }

    // --- 7. Failures and limits ---------------------------------------------------------------------------------------

    public function testAfterARefusalTheServerLeavesGameUpcAloneForAWhile(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGameUpcApi::lookAnswersText('{"status":"error","message":"Overloaded"}', 429);

        $first = $this->lookup($client, $ada, self::GLOOMHAVEN);
        self::assertResponseStatusCodeSame(503);
        self::assertSame('metadata_quota_exhausted', $first['error'] ?? null);

        $this->lookup($client, $ada, self::TINY_TOWNS);
        self::assertResponseStatusCodeSame(503);
        $this->choose($client, $ada, self::TINY_TOWNS, 265736);
        self::assertResponseStatusCodeSame(503);
        self::assertSame(1, FakeGameUpcApi::calls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testAServerErrorIsTriedThreeTimesAndThenFails(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        for ($i = 0; $i < 3; ++$i) {
            FakeGameUpcApi::lookAnswersText('oops', 500);
        }

        $body = $this->lookup($client, $ada, self::GLOOMHAVEN);

        self::assertResponseStatusCodeSame(502);
        self::assertSame('metadata_failed', $body['error'] ?? null);
        self::assertSame(3, FakeGameUpcApi::calls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testAnAccountPastItsShareOfLooksIsCutAndTheCacheStillServesIt(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        self::getContainer()->get('limiter.gameupc_account')->create($this->user('ada@example.com')->getId()->toRfc4122())->consume(50);
        FakeGameUpcApi::lookAnswers(FakeGameUpcApi::verified());

        $refused = $this->lookup($client, $ada, self::GLOOMHAVEN);
        self::assertResponseStatusCodeSame(429);
        self::assertSame('too_many_requests', $refused['error'] ?? null);

        $this->lookup($client, $grace, self::GLOOMHAVEN);
        self::assertResponseStatusCodeSame(200);
        $this->lookup($client, $ada, self::GLOOMHAVEN);
        self::assertResponseStatusCodeSame(200);
        self::assertSame(1, FakeGameUpcApi::calls());
    }

    public function testAnAccountPastItsShareOfChoicesIsCutAndTheOthersAreNot(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        self::getContainer()->get('limiter.gameupc_choice_account')->create($this->user('ada@example.com')->getId()->toRfc4122())->consume(30);
        FakeGameUpcApi::choiceAnswers();

        $refused = $this->choose($client, $ada, self::TINY_TOWNS, 265736);
        self::assertResponseStatusCodeSame(429);
        self::assertSame('too_many_requests', $refused['error'] ?? null);

        $this->choose($client, $grace, self::TINY_TOWNS, 265736);
        self::assertResponseStatusCodeSame(204);
        self::assertSame(1, FakeGameUpcApi::calls());
    }

    public function testWhenTheServersDayOfChoicesIsSpentNoMoreAreSent(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        self::getContainer()->get('limiter.gameupc_choices')->create('server')->consume(200);

        $body = $this->choose($client, $ada, self::TINY_TOWNS, 265736);

        self::assertResponseStatusCodeSame(503);
        self::assertSame('metadata_quota_exhausted', $body['error'] ?? null);
        self::assertSame(0, FakeGameUpcApi::calls());
    }

    public function testTheBarcodeReachesNoLogEvenWhenSomethingGoesWrong(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGameUpcApi::lookAnswersText('{}', 429);
        $this->lookup($client, $ada, self::GLOOMHAVEN);
        $this->clearBrake();
        for ($i = 0; $i < 3; ++$i) {
            FakeGameUpcApi::lookAnswersText('x', 500);
        }
        $this->lookup($client, $ada, '4006381333931');
        FakeGameUpcApi::lookFails(new TransportException('Could not reach https://api.gameupc.com/v1/upc/96385074'));
        $this->lookup($client, $ada, '96385074');
        $this->lookup($client, $ada, '019962194718');
        FakeGameUpcApi::choiceAnswers(500);
        FakeGameUpcApi::choiceAnswers(500);
        FakeGameUpcApi::choiceAnswers(500);
        $this->choose($client, $ada, '10012345678902', 265736);

        self::assertNotSame('', RecordingLogger::everything(), 'the logger must be the one the test reads');
        foreach ([self::GLOOMHAVEN, '4006381333931', '96385074', '019962194718', '10012345678902'] as $code) {
            self::assertStringNotContainsString($code, RecordingLogger::everything());
        }
    }

    // --- helpers ------------------------------------------------------------------------------------------------------

    private function world(): KernelBrowser
    {
        $_ENV['GAMEUPC_API_KEY'] = $_SERVER['GAMEUPC_API_KEY'] = self::KEY;
        self::ensureKernelShutdown();
        $client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();
        $this->clearBrake();
        FakeGameUpcApi::reset();
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

    private function gameUpcIdOf(string $email): string
    {
        return self::getContainer()->get(GameUpcUserId::class)->of($this->user($email)->getId()->toRfc4122());
    }

    /** @return list<mixed> what `GET /v1/server` says it looks up */
    private function offered(KernelBrowser $client): array
    {
        $metadata = $this->decode($client)['metadata'] ?? [];

        return \is_array($metadata) ? array_values($metadata) : [];
    }

    /**
     * @param array{headers: array<string, string>} $session
     *
     * @return array<string, mixed>
     */
    private function lookup(KernelBrowser $client, array $session, string $code): array
    {
        $this->raw($client, $session, $code);

        return $this->decode($client);
    }

    /** @param array{headers: array<string, string>} $session */
    private function raw(KernelBrowser $client, array $session, string $code): string
    {
        $client->request('POST', '/v1/metadata/gameupc/lookup', server: $session['headers'] + ['CONTENT_TYPE' => 'application/json'], content: json_encode(['code' => $code], \JSON_THROW_ON_ERROR));

        return (string) $client->getResponse()->getContent();
    }

    /**
     * @param array{headers: array<string, string>} $session
     *
     * @return array<string, mixed>
     */
    private function choose(KernelBrowser $client, array $session, string $code, int $gameId): array
    {
        $client->request('POST', '/v1/metadata/gameupc/choice', server: $session['headers'] + ['CONTENT_TYPE' => 'application/json'], content: json_encode(['code' => $code, 'external_id' => $gameId], \JSON_THROW_ON_ERROR));

        return $this->decode($client);
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

    private function entry(string $code): MetadataCacheEntry
    {
        return $this->entityManager()->getRepository(MetadataCacheEntry::class)->findOneBy(['provider' => 'gameupc', 'code' => $code]) ?? self::fail('No row.');
    }

    private function cacheRows(): int
    {
        $count = $this->entityManager()->getConnection()->fetchOne('SELECT COUNT(*) FROM metadata_cache');

        return is_numeric($count) ? (int) $count : self::fail('Not a count.');
    }

    /** @return list<string> */
    private function codes(): array
    {
        return $this->strings($this->entityManager()->getConnection()->fetchFirstColumn('SELECT code FROM metadata_cache ORDER BY code'));
    }

    /** @return list<string> */
    private function cacheColumns(): array
    {
        return $this->strings($this->entityManager()->getConnection()->fetchFirstColumn(
            "SELECT column_name FROM information_schema.columns WHERE table_name = 'metadata_cache' ORDER BY ordinal_position"
        ));
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
    private function allCacheRowsAsText(): array
    {
        $rows = $this->entityManager()->getConnection()->fetchAllAssociative('SELECT * FROM metadata_cache');

        return array_map(static fn (array $row): string => json_encode($row, \JSON_THROW_ON_ERROR), $rows);
    }

    /** @param array<string, mixed> $payload */
    private function store(string $code, bool $found, array $payload): void
    {
        $this->entityManager()->persist(new MetadataCacheEntry('gameupc', $code, $found, $payload, new \DateTimeImmutable()));
        $this->entityManager()->flush();
    }
}
