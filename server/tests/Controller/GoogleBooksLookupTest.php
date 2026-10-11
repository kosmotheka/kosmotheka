<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Entity\ActivityEntry;
use App\Entity\MetadataCacheEntry;
use App\Entity\User;
use App\Security\RequestLimiter;
use App\Tests\Support\DeviceAuthTestTrait;
use App\Tests\Support\FakeGoogleBooksApi;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\HttpClient\Exception\TransportException;

/**
 * Looking an ISBN up in Google Books through the server (7.8, D83). The real provider runs over a scripted http client.
 * Hostile cases: no session, a blocked account, something that is not an ISBN, the key leaking, a Google that sends
 * rubbish or says no, an account (or the whole server) past its limit, and the cache remembering who asked.
 */
final class GoogleBooksLookupTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string KEY = 'SENTINEL-google-books-key-7f3a';
    private const string INFERNO = '9788408176039';
    private const string OTHER = '9780140328721';

    protected function tearDown(): void
    {
        $_ENV['GOOGLE_BOOKS_API_KEY'] = $_SERVER['GOOGLE_BOOKS_API_KEY'] = '';
        FakeGoogleBooksApi::reset();
        parent::tearDown();
    }

    // --- The way -----------------------------------------------------------------------------------------------------

    public function testAnAccountGetsTheFixedShapeAndTheKeyTravelsInAHeader(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGoogleBooksApi::found(FakeGoogleBooksApi::volume());

        $body = $this->lookup($client, $ada, self::INFERNO);

        self::assertResponseStatusCodeSame(200);
        self::assertSame([
            'found' => true,
            'title' => 'Inferno',
            'subtitle' => 'A novel',
            'authors' => ['Dan Brown'],
            'publisher' => 'Planeta',
            'published_year' => 2013,
            'language' => 'es',
            'cover_url' => 'https://books.google.com/books/content?id=abc123&printsec=frontcover&img=1&zoom=1&source=gbs_api',
            'external_id' => 'abc123',
        ], $body);
        self::assertSame(1, FakeGoogleBooksApi::calls());
        $request = FakeGoogleBooksApi::$requests[0];
        self::assertSame(['X-Goog-Api-Key: '.self::KEY], $request['headers']['x-goog-api-key'] ?? null);
        self::assertStringStartsWith('https://www.googleapis.com/books/v1/volumes?', $request['url']);
        self::assertStringNotContainsString(self::KEY, $request['url']);
        self::assertStringContainsString('isbn:'.self::INFERNO, $request['url']);
    }

    public function testNotFoundIsAnAnswerAndIsRemembered(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGoogleBooksApi::notFound();

        $first = $this->lookup($client, $ada, self::INFERNO);
        $second = $this->lookup($client, $ada, self::INFERNO);

        self::assertResponseStatusCodeSame(200);
        self::assertFalse($first['found']);
        self::assertSame($first, $second);
        self::assertNull($first['title']);
        self::assertSame([], $first['authors']);
        self::assertSame(1, FakeGoogleBooksApi::calls());
    }

    public function testTheServerSaysItOffersGoogleBooksOnlyWithAKey(): void
    {
        $client = $this->world();
        $client->request('GET', '/v1/server');
        self::assertSame(['google_books'], $this->decode($client)['metadata'] ?? null);

        $_ENV['GOOGLE_BOOKS_API_KEY'] = $_SERVER['GOOGLE_BOOKS_API_KEY'] = '';
        self::ensureKernelShutdown();
        $client = self::createClient();
        $client->request('GET', '/v1/server');
        self::assertSame([], $this->decode($client)['metadata'] ?? null);
    }

    public function testWithoutAKeyTheRouteIsUnavailableAndNobodyIsAsked(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $_ENV['GOOGLE_BOOKS_API_KEY'] = $_SERVER['GOOGLE_BOOKS_API_KEY'] = '';
        self::ensureKernelShutdown();
        $client = self::createClient();

        $body = $this->lookup($client, $ada, self::INFERNO);

        self::assertResponseStatusCodeSame(404);
        self::assertSame('metadata_unavailable', $body['error'] ?? null);
        self::assertSame(0, FakeGoogleBooksApi::calls());
    }

    // --- 1. Who may ask -----------------------------------------------------------------------------------------------

    public function testWithoutASessionThereIsNoAnswerAndNoCall(): void
    {
        $client = $this->world();

        $client->request('GET', '/v1/metadata/google-books/isbn/'.self::INFERNO);

        self::assertResponseStatusCodeSame(401);
        self::assertSame(0, FakeGoogleBooksApi::calls());
    }

    public function testAnInvalidTokenGetsNoAnswerAndNoCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $headers = $this->deviceHeaders($ada['device_id'], 'not-a-token');

        $this->lookup($client, ['headers' => $headers] + $ada, self::INFERNO);

        self::assertResponseStatusCodeSame(401);
        self::assertSame(0, FakeGoogleBooksApi::calls());
    }

    public function testAnAccountThatIsNoLongerConfirmedGetsNoAnswerAndNoCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $this->entityManager()->createQuery(\sprintf('UPDATE %s u SET u.confirmedAt = NULL', User::class))->execute();

        $body = $this->lookup($client, $ada, self::INFERNO);

        self::assertResponseStatusCodeSame(403);
        self::assertSame('account_not_confirmed', $body['error'] ?? null);
        self::assertSame(0, FakeGoogleBooksApi::calls());
    }

    public function testABlockedAccountGetsNoAnswerAndNoCall(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $this->user('ada@example.com')->block(new \DateTimeImmutable());
        $this->entityManager()->flush();

        $body = $this->lookup($client, $ada, self::INFERNO);

        self::assertResponseStatusCodeSame(403);
        self::assertSame('account_blocked', $body['error'] ?? null);
        self::assertSame(0, FakeGoogleBooksApi::calls());
    }

    // --- 2. What is not an ISBN ----------------------------------------------------------------------------------------

    /** @return iterable<string, array{string}> */
    public static function notIsbns(): iterable
    {
        yield 'letters' => ['abcdefghijklm'];
        yield 'a bad check digit' => ['9788408176038'];
        yield 'too short' => ['97884081'];
        yield 'not a book prefix' => ['1234567890123'];
        yield 'dots' => ['..'];
        yield 'a line break' => ["9788408176039\n"];
        yield 'a line break inside' => ["97884081\n76039"];
        yield 'very long' => [str_repeat('9', 5000)];
        yield 'a query' => ['9788408176039?key=1'];
        yield 'the key shape' => ['AIzaSyA-b'];
        yield 'an injection' => ["978840817603' OR '1'='1"];
        yield 'x in the middle' => ['84081X7603'];
    }

    #[DataProvider('notIsbns')]
    public function testSomethingThatIsNotAnIsbnIsRefusedWithoutGoogleAndWithoutARow(string $isbn): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        $body = $this->lookup($client, $ada, $isbn);

        self::assertResponseStatusCodeSame(422);
        self::assertSame('invalid_isbn', $body['error'] ?? null);
        self::assertSame(0, FakeGoogleBooksApi::calls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testASlashIsNotAnIsbnEitherAndNothingIsAsked(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        $client->request('GET', '/v1/metadata/google-books/isbn/978/8408176039', server: $ada['headers']);

        self::assertResponseStatusCodeSame(404);
        self::assertSame(0, FakeGoogleBooksApi::calls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testAnIsbn10AndHyphensGoToTheSameThirteenDigitsAndOneRow(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGoogleBooksApi::found(FakeGoogleBooksApi::volume());

        $this->lookup($client, $ada, '840817603X');
        $this->lookup($client, $ada, '978-84-08-17603-9');
        $this->lookup($client, $ada, self::INFERNO);

        self::assertSame(1, FakeGoogleBooksApi::calls());
        self::assertStringContainsString('isbn:'.self::INFERNO, FakeGoogleBooksApi::$requests[0]['url']);
        self::assertSame([self::INFERNO], $this->cachedCodes());
    }

    // --- 3. The key never shows --------------------------------------------------------------------------------------

    public function testTheKeyIsNowhereInWhatTheServerAnswersOrStores(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $seen = [];

        // Google echoes the key in every error body, the way a badly configured proxy might.
        foreach ([429, 403, 500, 404] as $status) {
            // A 500 is asked again (three times at most): every answer carries the key.
            foreach (500 === $status ? [1, 2, 3] : [1] as $_) {
                FakeGoogleBooksApi::answer($status, '{"error":"bad key '.self::KEY.'"}');
            }
            $this->clearBrake();
            $seen[] = $this->lookupRaw($client, $ada, self::INFERNO);
        }
        FakeGoogleBooksApi::failWith(new TransportException('Could not resolve host for https://www.googleapis.com/books/v1/volumes?key='.self::KEY));
        $this->clearBrake();
        $seen[] = $this->lookupRaw($client, $ada, self::INFERNO);
        FakeGoogleBooksApi::found(FakeGoogleBooksApi::volume());
        $this->clearBrake();
        $seen[] = $this->lookupRaw($client, $ada, self::INFERNO);

        self::assertSame(8, FakeGoogleBooksApi::calls());
        foreach ($seen as $answer) {
            self::assertStringNotContainsString(self::KEY, $answer);
        }
        foreach ($this->allCacheRowsAsText() as $row) {
            self::assertStringNotContainsString(self::KEY, $row);
        }
    }

    // --- 4. A Google that sends rubbish ------------------------------------------------------------------------------

    public function testRubbishFromGoogleReachesTheAppOnlyAsTheFixedShape(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGoogleBooksApi::found([
            'id' => ['not', 'a', 'string'],
            'volumeInfo' => [
                'title' => str_repeat('T', 100_000),
                'subtitle' => ['an', 'array'],
                'authors' => array_fill(0, 5_000, 'Someone'),
                'publisher' => 42,
                'publishedDate' => 'long ago',
                'language' => '<script>alert(1)</script>',
                'imageLinks' => ['thumbnail' => 'https://evil.example/cover.jpg'],
                'extra' => str_repeat('x', 1000),
            ],
        ]);

        $body = $this->lookup($client, $ada, self::INFERNO);

        self::assertResponseStatusCodeSame(200);
        self::assertSame(['found', 'title', 'subtitle', 'authors', 'publisher', 'published_year', 'language', 'cover_url', 'external_id'], array_keys($body));
        self::assertTrue($body['found']);
        self::assertIsString($body['title']);
        self::assertLessThanOrEqual(300, mb_strlen($body['title']));
        self::assertNull($body['subtitle']);
        self::assertIsArray($body['authors']);
        self::assertLessThanOrEqual(20, \count($body['authors']));
        self::assertNull($body['publisher']);
        self::assertNull($body['published_year']);
        self::assertNull($body['language']);
        self::assertNull($body['cover_url']);
        self::assertNull($body['external_id']);
    }

    public function testAGoogleThatAnswersNonsenseIsAFailureAndLeavesNoRow(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        foreach (['<html>hello</html>', '[1,2,3', 'null', '"text"', str_repeat('{"a":1,', 400_000)] as $nonsense) {
            FakeGoogleBooksApi::answer(200, $nonsense);
            $body = $this->lookup($client, $ada, self::INFERNO);
            self::assertResponseStatusCodeSame(502, 'for '.substr($nonsense, 0, 20));
            self::assertSame('metadata_failed', $body['error'] ?? null);
        }

        self::assertSame(0, $this->cacheRows());
    }

    // --- 5. Google says no --------------------------------------------------------------------------------------------

    #[DataProvider('quotaStatuses')]
    public function testGoogleSayingNoOnQuotaIsReportedAndNothingIsStored(int $status): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGoogleBooksApi::answer($status, '{"error":{"code":'.$status.'}}');

        $body = $this->lookup($client, $ada, self::INFERNO);

        self::assertResponseStatusCodeSame(503);
        self::assertSame('metadata_quota_exhausted', $body['error'] ?? null);
        self::assertSame(0, $this->cacheRows());
    }

    /** @return iterable<string, array{int}> */
    public static function quotaStatuses(): iterable
    {
        yield 'too many requests' => [429];
        yield 'forbidden' => [403];
    }

    public function testAnEnormousAnswerFromGoogleIsAFailureAndLeavesNoRow(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGoogleBooksApi::found(FakeGoogleBooksApi::volume(str_repeat('T', 1_000_000)));

        $body = $this->lookup($client, $ada, self::INFERNO);

        self::assertResponseStatusCodeSame(502);
        self::assertSame('metadata_failed', $body['error'] ?? null);
        self::assertSame(0, $this->cacheRows());
    }

    public function testAfterAQuotaAnswerTheServerStopsAskingGoogleForAWhile(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGoogleBooksApi::answer(429, '{}');

        $this->lookup($client, $ada, self::INFERNO);
        for ($try = 0; $try < 5; ++$try) {
            $body = $this->lookup($client, $ada, self::OTHER);
            self::assertSame('metadata_quota_exhausted', $body['error'] ?? null);
        }

        self::assertSame(1, FakeGoogleBooksApi::calls());
        self::assertGreaterThan(0, (int) $client->getResponse()->headers->get('Retry-After'));

        // Once the pause is over, the server tries again.
        $this->clearBrake();
        FakeGoogleBooksApi::found(FakeGoogleBooksApi::volume());
        $this->lookup($client, $ada, self::OTHER);
        self::assertResponseStatusCodeSame(200);
        self::assertSame(2, FakeGoogleBooksApi::calls());
    }

    public function testAGoogleThatFailsOrIsSlowIsAFailureAndLeavesNoRow(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        foreach ([1, 2, 3] as $_) {
            FakeGoogleBooksApi::answer(500, 'oops');
        }
        FakeGoogleBooksApi::failWith(new TransportException('Idle timeout reached'));

        foreach ([1, 2] as $_) {
            $body = $this->lookup($client, $ada, self::INFERNO);
            self::assertResponseStatusCodeSame(502);
            self::assertSame('metadata_failed', $body['error'] ?? null);
        }

        self::assertSame(0, $this->cacheRows());
    }

    public function testAGoogleThatBreaksOnceIsAskedAgainAndTheBookArrives(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGoogleBooksApi::answer(503, '{"error":{"code":503,"message":"Service temporarily unavailable."}}');
        FakeGoogleBooksApi::found(FakeGoogleBooksApi::volume());

        $body = $this->lookup($client, $ada, self::INFERNO);

        self::assertResponseIsSuccessful();
        self::assertSame('Inferno', $body['title'] ?? null);
        self::assertSame(2, FakeGoogleBooksApi::calls());
        self::assertSame(1, $this->cacheRows());
    }

    public function testAGoogleThatKeepsBreakingIsAskedThreeTimesAndNoMore(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        foreach ([503, 500, 503] as $status) {
            FakeGoogleBooksApi::answer($status, 'oops');
        }
        // Never reached: three is the most one lookup may cost.
        FakeGoogleBooksApi::found(FakeGoogleBooksApi::volume());

        $body = $this->lookup($client, $ada, self::INFERNO);

        self::assertResponseStatusCodeSame(502);
        self::assertSame('metadata_failed', $body['error'] ?? null);
        self::assertSame(3, FakeGoogleBooksApi::calls());
        self::assertSame(0, $this->cacheRows());
    }

    public function testOnlyAGoogleThatBrokeIsAskedAgainNotASlowOneNorARefusal(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');

        // Slow: asking again would make the person wait three times as long.
        FakeGoogleBooksApi::failWith(new TransportException('Idle timeout reached'));
        $this->lookup($client, $ada, self::INFERNO);
        self::assertResponseStatusCodeSame(502);
        self::assertSame(1, FakeGoogleBooksApi::calls());

        // A refusal of the request itself (a bad key is a 400): the same request gets the same answer.
        FakeGoogleBooksApi::answer(400, '{"error":{"code":400}}');
        $this->lookup($client, $ada, self::INFERNO);
        self::assertResponseStatusCodeSame(502);
        self::assertSame(2, FakeGoogleBooksApi::calls());
    }

    public function testEveryAttemptSpendsTheServersDay(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGoogleBooksApi::answer(503, 'oops');
        FakeGoogleBooksApi::answer(503, 'oops');
        FakeGoogleBooksApi::found(FakeGoogleBooksApi::volume());

        $this->lookup($client, $ada, self::INFERNO);

        self::assertResponseIsSuccessful();
        $left = self::getContainer()->get('limiter.google_books_calls')->create('server')->consume(0)->getRemainingTokens();
        self::assertSame(900 - 3, $left);
    }

    // --- 6. Limits ----------------------------------------------------------------------------------------------------

    public function testAnAccountPastItsLimitIsCutAndTheOthersAreNot(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        FakeGoogleBooksApi::found(FakeGoogleBooksApi::volume());
        $limiter = self::getContainer()->get(RequestLimiter::class);
        for ($i = 0; $i < 599; ++$i) {
            $limiter->consumeMetadata($this->user('ada@example.com')->getId()->toRfc4122());
        }

        $this->lookup($client, $ada, self::INFERNO);
        self::assertResponseStatusCodeSame(200);
        $body = $this->lookup($client, $ada, self::INFERNO); // from the cache, and still counted
        self::assertResponseStatusCodeSame(429);
        self::assertSame('too_many_requests', $body['error'] ?? null);
        self::assertGreaterThan(0, (int) $client->getResponse()->headers->get('Retry-After'));

        $this->lookup($client, $grace, self::INFERNO);
        self::assertResponseStatusCodeSame(200);
    }

    public function testWhenTheServersDayIsSpentTheCacheKeepsServing(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        FakeGoogleBooksApi::found(FakeGoogleBooksApi::volume());
        $this->lookup($client, $ada, self::INFERNO);
        $limiter = self::getContainer()->get(RequestLimiter::class);
        for ($i = 0; $i < 899; ++$i) {
            $limiter->consumeGoogleBooksCall(); // with the one made above, the 900 of the day
        }

        $cached = $this->lookup($client, $ada, self::INFERNO);
        self::assertResponseStatusCodeSame(200);
        self::assertSame('Inferno', $cached['title']);

        $other = $this->lookup($client, $ada, self::OTHER);
        self::assertResponseStatusCodeSame(503);
        self::assertSame('metadata_quota_exhausted', $other['error'] ?? null);
        self::assertSame(1, FakeGoogleBooksApi::calls());
        self::assertSame('Inferno', $this->lookup($client, $ada, self::INFERNO)['title']);
    }

    // --- 7. Nobody is remembered -------------------------------------------------------------------------------------

    public function testTwoAccountsAskingTheSameIsbnCostOneCallAndNoRowSaysWho(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $grace = $this->account($client, 'grace@example.com');
        FakeGoogleBooksApi::found(FakeGoogleBooksApi::volume());
        $activity = $this->entityManager()->getRepository(ActivityEntry::class)->count([]);

        $this->lookup($client, $ada, self::INFERNO);
        $this->lookup($client, $grace, self::INFERNO);

        self::assertSame(1, FakeGoogleBooksApi::calls());
        self::assertSame(1, $this->cacheRows());
        self::assertSame(['id', 'provider', 'code', 'found', 'payload', 'fetched_at'], $this->cacheColumns());
        $row = implode(' ', $this->allCacheRowsAsText());
        self::assertStringNotContainsString('ada@example.com', $row);
        self::assertStringNotContainsString('grace@example.com', $row);
        self::assertStringNotContainsString($this->user('ada@example.com')->getId()->toRfc4122(), $row);
        self::assertSame($activity, $this->entityManager()->getRepository(ActivityEntry::class)->count([]));
    }

    // --- The cache's age ------------------------------------------------------------------------------------------------

    public function testWhatWasFoundIsKeptThirtyDaysAndWhatWasNotOneDay(): void
    {
        $client = $this->world();
        $ada = $this->account($client, 'ada@example.com');
        $now = new \DateTimeImmutable();
        $this->store(self::INFERNO, true, ['title' => 'Old but fresh'], $now->modify('-29 days'));
        $this->store(self::OTHER, false, [], $now->modify('-23 hours'));
        FakeGoogleBooksApi::found(FakeGoogleBooksApi::volume('New', 'n1'));

        self::assertSame('Old but fresh', $this->lookup($client, $ada, self::INFERNO)['title']);
        self::assertFalse($this->lookup($client, $ada, self::OTHER)['found']);
        self::assertSame(0, FakeGoogleBooksApi::calls());

        $this->entityManager()->createQuery(\sprintf('UPDATE %s e SET e.fetchedAt = :at WHERE e.code = :code', MetadataCacheEntry::class))
            ->setParameter('at', $now->modify('-31 days'))->setParameter('code', self::INFERNO)->execute();
        $this->entityManager()->createQuery(\sprintf('UPDATE %s e SET e.fetchedAt = :at WHERE e.code = :code', MetadataCacheEntry::class))
            ->setParameter('at', $now->modify('-25 hours'))->setParameter('code', self::OTHER)->execute();
        $this->entityManager()->clear();

        FakeGoogleBooksApi::found(FakeGoogleBooksApi::volume('Second', 'n2'));
        self::assertSame('New', $this->lookup($client, $ada, self::INFERNO)['title']);
        self::assertSame('Second', $this->lookup($client, $ada, self::OTHER)['title']);
        self::assertSame(2, FakeGoogleBooksApi::calls());
        self::assertSame(2, $this->cacheRows());
    }

    // --- helpers ------------------------------------------------------------------------------------------------------

    private function world(): KernelBrowser
    {
        $_ENV['GOOGLE_BOOKS_API_KEY'] = $_SERVER['GOOGLE_BOOKS_API_KEY'] = self::KEY;
        self::ensureKernelShutdown();
        $client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();
        $this->clearBrake();
        FakeGoogleBooksApi::reset();

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

    /**
     * @param array{headers: array<string, string>} $session
     *
     * @return array<string, mixed>
     */
    private function lookup(KernelBrowser $client, array $session, string $isbn): array
    {
        $this->lookupRaw($client, $session, $isbn);

        return $this->decode($client);
    }

    /** @param array{headers: array<string, string>} $session */
    private function lookupRaw(KernelBrowser $client, array $session, string $isbn): string
    {
        $client->request('GET', '/v1/metadata/google-books/isbn/'.rawurlencode($isbn), server: $session['headers']);

        return (string) $client->getResponse()->getContent();
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

    private function cacheRows(): int
    {
        return $this->numberFrom('SELECT COUNT(*) FROM metadata_cache');
    }

    private function numberFrom(string $sql): int
    {
        $count = $this->entityManager()->getConnection()->fetchOne($sql);

        return is_numeric($count) ? (int) $count : self::fail('Not a count.');
    }

    /** @return list<string> */
    private function cachedCodes(): array
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
    private function store(string $code, bool $found, array $payload, \DateTimeImmutable $at): void
    {
        $this->entityManager()->persist(new MetadataCacheEntry('google_books', $code, $found, $payload, $at));
        $this->entityManager()->flush();
    }
}
