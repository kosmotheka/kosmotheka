<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Entity\ActivityEntry;
use App\Entity\User;
use App\Enum\ActivityKind;
use App\Repository\ActivityEntryRepository;
use App\Security\AccountPurger;
use App\Sync\Invitation\EmailFingerprint;
use App\Tests\Support\DeviceAuthTestTrait;
use Doctrine\DBAL\Connection;
use Doctrine\ORM\EntityManagerInterface;
use OTPHP\TOTP;
use Symfony\Bundle\FrameworkBundle\Console\Application;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\Console\Tester\CommandTester;
use Symfony\Component\Mime\Email;
use Symfony\Component\Routing\RouterInterface;
use Symfony\Component\Uid\Uuid;

/**
 * The activity log of the account (D66, 7.7): what is written, by whom, from where, and what the log never holds.
 * The library and invitation lines are tested with their world, in InvitationControllerTest. The hostile cases come
 * first, as in the rest of the server.
 */
final class ActivityLogTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string PASSWORD = 'correct horse battery';

    private KernelBrowser $client;

    protected function setUp(): void
    {
        $this->client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();
    }

    // ------------------------------------------------------------ hostile ---

    /** Hostile 3: a user agent of megabytes, with line breaks and control characters, is cut and cleaned. */
    public function testAHugeUserAgentWithControlCharactersIsCutAndCleaned(): void
    {
        $this->registerAndConfirmForLogin($this->client, 'ada@example.com', self::PASSWORD);
        $agent = "Kosmotheka/1.0\r\n\x1b[31mEVIL\x00\x07 ".str_repeat('é', 3_000_000);
        $this->client->setServerParameter('HTTP_USER_AGENT', $agent);

        $session = $this->loginDevice($this->client, 'ada@example.com', self::PASSWORD, Uuid::v4()->toRfc4122(), 'Phone');

        $line = $this->lineOf(ActivityKind::LoginSucceeded);
        $stored = $line->getUserAgent();
        self::assertNotNull($stored);
        self::assertLessThanOrEqual(255, mb_strlen($stored));
        self::assertStringStartsWith('Kosmotheka/1.0', $stored);
        self::assertSame(0, preg_match('/[\x00-\x1F\x7F]/', $stored), 'no control characters');
        self::assertTrue(mb_check_encoding($stored, 'UTF-8'));
        self::assertSame(200, $this->client->getResponse()->getStatusCode());
        self::assertNotSame('', $session['token']);

        // And the console that lists it shows it as plain text.
        $tester = $this->listCommand();
        $tester->execute([]);
        self::assertSame(0, preg_match('/\x1b|\x07|\x00/', $tester->getDisplay()));
    }

    /** Hostile 4: a forged `X-Forwarded-For` does not choose the address that is written. */
    public function testAForgedForwardedForDoesNotChooseTheAddress(): void
    {
        $this->registerAndConfirmForLogin($this->client, 'ada@example.com', self::PASSWORD);

        // A client that reaches the server directly, saying it is somebody else.
        $this->client->setServerParameter('REMOTE_ADDR', '203.0.113.9');
        $this->client->setServerParameter('HTTP_X_FORWARDED_FOR', '6.6.6.6');
        $this->loginDevice($this->client, 'ada@example.com', self::PASSWORD, Uuid::v4()->toRfc4122(), 'Phone');
        self::assertSame('203.0.113.9', $this->lineOf(ActivityKind::LoginSucceeded)->getIp());

        // Behind the trusted proxy (127.0.0.1, `TRUSTED_PROXIES`), which appends the address it saw: what the client
        // forged on the left of it is not believed either.
        $this->client->setServerParameter('REMOTE_ADDR', '127.0.0.1');
        $this->client->setServerParameter('HTTP_X_FORWARDED_FOR', '6.6.6.6, 198.51.100.7');
        $this->loginDevice($this->client, 'ada@example.com', self::PASSWORD, Uuid::v4()->toRfc4122(), 'Tablet');
        $lines = $this->linesOf(ActivityKind::LoginSucceeded);
        self::assertSame('198.51.100.7', $lines[0]->getIp());
    }

    /** Hostile 5: the lines of an account that is deleted stay, with its id and the fingerprint of its email, never the email. */
    public function testTheLinesOfADeletedAccountStayWithTheFingerprintAndWithoutTheEmail(): void
    {
        $session = $this->signInNewDevice($this->client, 'ada@example.com', self::PASSWORD);
        $id = $this->userId('ada@example.com');
        $this->client->request('DELETE', '/v1/account', server: ['CONTENT_TYPE' => 'application/json'] + $session['headers'], content: json_encode(['password' => self::PASSWORD], \JSON_THROW_ON_ERROR));
        self::assertSame(204, $this->client->getResponse()->getStatusCode());

        $lines = $this->linesOf(null);
        self::assertContains(ActivityKind::AccountDeleted, array_map(static fn (ActivityEntry $line): ActivityKind => $line->getKind(), $lines));
        $fingerprint = self::getContainer()->get(EmailFingerprint::class)->of('ada@example.com');
        foreach ($lines as $line) {
            self::assertSame($id, $line->getAccountId()?->toRfc4122());
            self::assertSame($fingerprint, $line->getAccountEmailFingerprint(), 'it answers «was it this address?» without holding the address');
        }
        $tester = $this->listCommand();
        $tester->execute(['--account' => $id]);
        self::assertStringContainsString('account_deleted', $tester->getDisplay());
        self::assertStringNotContainsString('ada@example.com', $tester->getDisplay());
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM "user" WHERE email = ?', ['ada@example.com']));
    }

    /** Hostile 6: no route gives back a line of the log, to the account it is about or to anybody. */
    public function testNoRouteReturnsALineOfTheLog(): void
    {
        $this->client->setServerParameter('HTTP_USER_AGENT', 'MarkerAgent/9.9');
        $session = $this->signInNewDevice($this->client, 'ada@example.com', self::PASSWORD);
        self::assertGreaterThan(0, $this->scalar('SELECT COUNT(*) FROM activity_log WHERE user_agent = ?', ['MarkerAgent/9.9']));

        $router = self::getContainer()->get(RouterInterface::class);
        $seen = 0;
        foreach ($router->getRouteCollection() as $name => $route) {
            self::assertStringNotContainsString('activity', strtolower($route->getPath()), $name);
            if (!\in_array('GET', $route->getMethods(), true) || str_contains($route->getPath(), '{') || str_starts_with($name, '_')) {
                continue;
            }
            $this->client->request('GET', $route->getPath(), server: ['CONTENT_TYPE' => 'application/json'] + $session['headers']);
            self::assertStringNotContainsString('MarkerAgent', (string) $this->client->getResponse()->getContent(), $route->getPath());
            ++$seen;
        }
        self::assertGreaterThan(2, $seen);
    }

    /** Hostile 7: the purge deletes what is older than a year and nothing more recent. */
    public function testThePurgeDeletesWhatIsOlderThanAYearAndNothingNewer(): void
    {
        $em = self::getContainer()->get(EntityManagerInterface::class);
        $now = new \DateTimeImmutable('now', new \DateTimeZone('UTC'));
        foreach (['-366 days' => 'old', '-364 days' => 'recent', '-1 minute' => 'now'] as $age => $label) {
            $em->persist(new ActivityEntry(ActivityKind::LoginSucceeded, $now->modify($age), 'http', null, null, null, null, null, $label));
        }
        $em->flush();

        $result = self::getContainer()->get(AccountPurger::class)->purge();

        self::assertSame(1, $result['activity']);
        $left = self::getContainer()->get(Connection::class)->fetchFirstColumn('SELECT subject_id FROM activity_log ORDER BY occurred_at');
        self::assertSame(['recent', 'now'], $left);
    }

    /** Hostile 8: no password, code, token or email address in any line the flows of the account write. */
    public function testNoLineHoldsAPasswordACodeATokenOrAnEmail(): void
    {
        $secrets = [];
        $this->post('/v1/account/register', ['email' => 'ada@example.com', 'password' => self::PASSWORD, 'name' => 'Ada', 'locale' => 'en'], ['headers' => []]);
        $mail = self::getMailerMessage();
        \assert($mail instanceof Email);
        preg_match('/\d{6}/', (string) $mail->getTextBody(), $mailed);
        $secrets[] = $mailed[0] ?? self::fail('No code.');
        $this->post('/v1/account/confirm', ['email' => 'ada@example.com', 'code' => $mailed[0]], ['headers' => []]);
        $session = $this->loginDevice($this->client, 'ada@example.com', self::PASSWORD, Uuid::v4()->toRfc4122(), 'Phone');
        $secrets[] = $session['token'];
        $secrets[] = $session['secret_key'];
        // A wrong password, and a password change.
        $this->login('ada@example.com', 'wrong password 123');
        $this->post('/v1/account/password', ['current_password' => self::PASSWORD, 'new_password' => 'a brand new password'], $session);
        $this->post('/v1/auth/logout', null, $session);
        // The second factor.
        $session = $this->loginDevice($this->client, 'ada@example.com', 'a brand new password', Uuid::v4()->toRfc4122(), 'Tablet');
        $setup = $this->post('/v1/account/two-factor/setup', null, $session);
        $secret = $setup['body']['secret'] ?? null;
        \assert(\is_string($secret) && '' !== $secret);
        $code = TOTP::createFromSecret($secret)->now();
        $enabled = $this->post('/v1/account/two-factor/enable', ['code' => $code], $session);
        $backup = $enabled['body']['backup_codes'] ?? [];
        \assert(\is_array($backup));
        $secrets = [...$secrets, $secret, $code, ...array_filter($backup, \is_string(...)), self::PASSWORD, 'wrong password 123', 'a brand new password', $session['token']];

        $rows = self::getContainer()->get(Connection::class)->fetchAllAssociative('SELECT * FROM activity_log');
        self::assertGreaterThan(5, \count($rows));
        $everything = json_encode($rows, \JSON_THROW_ON_ERROR);
        foreach (array_filter($secrets) as $secretValue) {
            self::assertStringNotContainsString($secretValue, $everything);
        }
        self::assertStringNotContainsStringIgnoringCase('ada@example.com', $everything);
        self::assertStringNotContainsString('example.com', $everything);
    }

    // ------------------------------------------------------------- normal ---

    public function testTheAccountsOwnEventsAreWrittenWithTheServersAuthorTimeAndPlace(): void
    {
        $before = new \DateTimeImmutable('-2 seconds', new \DateTimeZone('UTC'));
        $this->client->setServerParameter('HTTP_USER_AGENT', 'Kosmotheka/1.2 (Android)');
        $this->client->setServerParameter('REMOTE_ADDR', '203.0.113.50');
        $session = $this->signInNewDevice($this->client, 'ada@example.com', self::PASSWORD);
        $user = $this->userId('ada@example.com');

        $this->post('/v1/auth/logout', null, $session);
        $this->login('ada@example.com', 'not the password');

        $kinds = array_map(static fn (ActivityEntry $line): string => $line->getKind()->value, array_reverse($this->linesOf(null)));
        self::assertSame(['account_registered', 'login_succeeded', 'device_signed_out', 'login_failed'], $kinds);
        foreach ($this->linesOf(null) as $line) {
            self::assertSame($user, $line->getAccountId()?->toRfc4122());
            self::assertSame('203.0.113.50', $line->getIp());
            self::assertSame('Kosmotheka/1.2 (Android)', $line->getUserAgent());
            self::assertSame('http', $line->getChannel());
            self::assertGreaterThanOrEqual($before, $line->getOccurredAt());
        }
        $login = $this->lineOf(ActivityKind::LoginSucceeded);
        self::assertSame($session['device_id'], $login->getDeviceId());
        self::assertNull($this->lineOf(ActivityKind::LoginFailed)->getDeviceId(), 'the device was only proved to hold its key, not to be the account\'s');
    }

    /**
     * The same write whether the account exists or not: a line only for accounts that exist would make a failed login
     * against one of them take longer, and tell which addresses have an account.
     */
    public function testAFailedLoginIsWrittenTheSameWhetherTheAccountExistsOrNot(): void
    {
        $this->client->setServerParameter('REMOTE_ADDR', '203.0.113.50');
        $this->login('nobody@example.com', 'not the password');

        $line = $this->lineOf(ActivityKind::LoginFailed);
        self::assertNull($line->getAccountId());
        self::assertNull($line->getAccountEmailFingerprint(), 'nothing of an address that is not an account\'s is kept');
        self::assertSame('203.0.113.50', $line->getIp());
    }

    public function testPasswordResetDeleteAndSecondFactorAreWritten(): void
    {
        $session = $this->signInNewDevice($this->client, 'ada@example.com', self::PASSWORD);
        $this->post('/v1/account/password', ['current_password' => self::PASSWORD, 'new_password' => 'a brand new password'], $session);
        $setup = $this->post('/v1/account/two-factor/setup', null, $session);
        $secret = $setup['body']['secret'] ?? null;
        \assert(\is_string($secret) && '' !== $secret);
        $enabled = $this->post('/v1/account/two-factor/enable', ['code' => TOTP::createFromSecret($secret)->now()], $session);
        $codes = $enabled['body']['backup_codes'] ?? null;
        \assert(\is_array($codes) && \is_string($codes[0] ?? null));
        $backup = $codes[0];
        $this->client->request('DELETE', '/v1/account/two-factor', server: ['CONTENT_TYPE' => 'application/json'] + $session['headers'], content: json_encode(['password' => 'a brand new password', 'code' => $backup], \JSON_THROW_ON_ERROR));
        self::assertSame(204, $this->client->getResponse()->getStatusCode(), (string) $this->client->getResponse()->getContent());

        $kinds = array_map(static fn (ActivityEntry $line): string => $line->getKind()->value, $this->linesOf(null));
        foreach (['password_changed', 'two_factor_enabled', 'two_factor_disabled'] as $expected) {
            self::assertContains($expected, $kinds);
        }
    }

    public function testBlockingFromTheConsoleIsWrittenWithNoAddress(): void
    {
        $this->signInNewDevice($this->client, 'ada@example.com', self::PASSWORD);
        $tester = new CommandTester(new Application(self::$kernel ?? self::bootKernel())->find('app:user:block'));
        $tester->execute(['email' => 'ada@example.com']);

        $line = $this->lineOf(ActivityKind::AccountBlocked);
        self::assertSame($this->userId('ada@example.com'), $line->getAccountId()?->toRfc4122());
        self::assertSame('console', $line->getChannel());
        self::assertNull($line->getIp());
    }

    public function testTheConsoleListsWithFiltersAndALimit(): void
    {
        $this->signInNewDevice($this->client, 'ada@example.com', self::PASSWORD);
        $this->signInNewDevice($this->client, 'bob@example.com', self::PASSWORD);
        $ada = $this->userId('ada@example.com');

        $all = $this->listCommand();
        $all->execute(['--limit' => '2']);
        self::assertStringContainsString('2 line(s) (the limit', $all->getDisplay());

        $one = $this->listCommand();
        $one->execute(['--account' => $ada, '--kind' => 'login_succeeded']);
        self::assertStringContainsString('1 line(s).', $one->getDisplay());
        self::assertStringContainsString($ada, $one->getDisplay());

        $none = $this->listCommand();
        $none->execute(['--since' => '2999-01-01']);
        self::assertStringContainsString('0 line(s).', $none->getDisplay());

        $wrong = $this->listCommand();
        self::assertSame(2, $wrong->execute(['--kind' => 'nonsense']));
    }

    // ------------------------------------------------------------ helpers ---

    private function listCommand(): CommandTester
    {
        return new CommandTester(new Application(self::$kernel ?? self::bootKernel())->find('app:activity:list'));
    }

    private function lineOf(ActivityKind $kind): ActivityEntry
    {
        return $this->linesOf($kind)[0] ?? self::fail("No line of $kind->value.");
    }

    /**
     * Newest first.
     *
     * @return list<ActivityEntry>
     */
    private function linesOf(?ActivityKind $kind): array
    {
        $em = self::getContainer()->get(EntityManagerInterface::class);
        $em->clear();

        return self::getContainer()->get(ActivityEntryRepository::class)->search(null, null, $kind, null, null, 100);
    }

    /** @param list<string> $params */
    private function scalar(string $sql, array $params = []): int
    {
        $count = self::getContainer()->get(Connection::class)->fetchOne($sql, $params);
        \assert(\is_int($count));

        return $count;
    }

    private function userId(string $email): string
    {
        $user = self::getContainer()->get(EntityManagerInterface::class)->getRepository(User::class)->findOneBy(['email' => $email]) ?? self::fail('No user.');

        return $user->getId()->toRfc4122();
    }

    /** A login with a wrong or right password, signed with a fresh key. */
    private function login(string $email, string $password): void
    {
        $this->loginDeviceUnchecked($email, $password);
    }

    private function loginDeviceUnchecked(string $email, string $password): void
    {
        $keyPair = sodium_crypto_sign_keypair();
        $deviceId = Uuid::v4()->toRfc4122();
        $challenge = $this->requestChallenge($this->client)['challenge'];
        $this->requestLogin(
            $this->client,
            $email,
            $password,
            $deviceId,
            'Phone',
            $this->encode(sodium_crypto_sign_publickey($keyPair)),
            $challenge,
            $this->sign($challenge, $deviceId, sodium_crypto_sign_secretkey($keyPair)),
        );
    }

    /**
     * @param array{headers: array<string, string>} $session
     * @param array<mixed>|null                     $body
     *
     * @return array{status: int, body: array<mixed>}
     */
    private function post(string $uri, ?array $body, array $session): array
    {
        $this->client->request('POST', $uri, server: ['CONTENT_TYPE' => 'application/json'] + $session['headers'], content: null === $body ? null : json_encode($body, \JSON_THROW_ON_ERROR));
        $content = (string) $this->client->getResponse()->getContent();
        $decoded = '' === $content ? [] : json_decode($content, true, flags: \JSON_THROW_ON_ERROR);
        \assert(\is_array($decoded));

        return ['status' => $this->client->getResponse()->getStatusCode(), 'body' => $decoded];
    }
}
