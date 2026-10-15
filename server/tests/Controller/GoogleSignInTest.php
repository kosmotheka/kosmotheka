<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Entity\ActivityEntry;
use App\Entity\Device;
use App\Entity\EmailCode;
use App\Entity\GoogleAttempt;
use App\Entity\User;
use App\Enum\EmailCodePurpose;
use App\Repository\UserRepository;
use App\Security\AccountPurger;
use App\Security\Google\GoogleIdentity;
use App\Security\TwoFactorSecretBox;
use App\Tests\Support\DeviceAuthTestTrait;
use App\Tests\Support\FakeGoogle;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\Uid\Uuid;

/**
 * «Continuar con Google» (D77) against a Google in memory. The way: the app starts with a secret that never leaves it
 * (`verifier`), the server does the OAuth and hands the browser back to the app with a one-use code, and the app
 * exchanges the code for its token with the proof of its device. Hostile cases first: a stolen code, a replayed one,
 * a code or state of another browser, an address Google does not confirm, an account somebody else prepared, a closed
 * server, a blocked account, a second factor, a server without Google.
 */
final class GoogleSignInTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string VARIABLE = 'REGISTRATION_ALLOWED_EMAILS';

    protected function tearDown(): void
    {
        $_ENV[self::VARIABLE] = $_SERVER[self::VARIABLE] = '';
        FakeGoogle::reset();
        parent::tearDown();
    }

    // --- The way ---------------------------------------------------------------------------------------------------

    public function testTheWholeWayGivesTheDeviceThatStartedItATokenAndMakesAnAccountWithNoPassword(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'Ada@Example.com', true, 'Ada Lovelace');

        $came = $this->comeBack($client, $this->start($client, 'es'));
        $session = $this->finish($client, $came);

        self::assertResponseStatusCodeSame(200);
        self::assertNotSame('', $this->body($client)['token'] ?? '');
        self::assertSame('ada@example.com', $this->userBody($client)['email'] ?? null);
        self::assertSame('Ada Lovelace', $this->userBody($client)['name'] ?? null);
        self::assertSame('es', $this->userBody($client)['locale'] ?? null);
        $user = $this->user('ada@example.com');
        self::assertTrue($user->isConfirmed());
        self::assertNull($user->getPassword(), 'a Google account has no password');
        self::assertNotNull($this->entityManager()->getRepository(Device::class)->findOneBy(['deviceId' => $session['device_id']]));
        // And the token is a token: it opens the authenticated API.
        $client->request('GET', '/v1/account', server: $this->deviceHeaders($session['device_id'], $this->tokenOf($client)));
    }

    public function testTheTokenOfTheWayOpensTheApi(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');
        $session = $this->finish($client, $this->comeBack($client, $this->start($client)));
        $token = $this->tokenOf($client);

        $client->request('GET', '/v1/account', server: $this->deviceHeaders($session['device_id'], $token));

        self::assertResponseStatusCodeSame(200);
        self::assertFalse($this->body($client)['has_password'] ?? true);
    }

    // --- 1. A stolen code is worth nothing ---------------------------------------------------------------------------

    public function testACodeStolenOnTheWayBackGivesNoTokenWithoutTheVerifier(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');
        $came = $this->comeBack($client, $this->start($client));

        foreach (['not-the-verifier', $this->newVerifier()] as $wrong) {
            $this->finish($client, ['verifier' => $wrong] + $came);
            self::assertNotSame(200, $client->getResponse()->getStatusCode());
            self::assertSame('invalid_google_code', $this->body($client)['error'] ?? null);
        }
        self::assertSame(0, $this->entityManager()->getRepository(Device::class)->count([]), 'no device was signed in');
    }

    public function testTheVerifierOfAnotherAttemptDoesNotOpenThisOne(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');
        $other = $this->start($client);
        $victim = $this->comeBack($client, $this->start($client));

        $this->finish($client, ['verifier' => $other['verifier']] + $victim);

        self::assertSame('invalid_google_code', $this->body($client)['error'] ?? null);
    }

    public function testAnAttemptThatIsGuessedAtGivesUpAfterFiveTries(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');
        $came = $this->comeBack($client, $this->start($client));

        for ($try = 0; $try < 5; ++$try) {
            $this->finish($client, ['verifier' => $this->newVerifier()] + $came);
        }
        $this->finish($client, $came);

        self::assertSame('invalid_google_code', $this->body($client)['error'] ?? null, 'even the right verifier: it was given up');
    }

    // --- 2. Used, expired --------------------------------------------------------------------------------------------

    public function testACodeUsedTwiceGivesNothingTheSecondTime(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');
        $came = $this->comeBack($client, $this->start($client));
        $this->finish($client, $came);
        self::assertResponseStatusCodeSame(200);

        $this->finish($client, $came);

        self::assertResponseStatusCodeSame(400);
        self::assertSame('invalid_google_code', $this->body($client)['error'] ?? null);
    }

    public function testAnExpiredCodeAndAnExpiredAttemptGiveNothing(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');

        $came = $this->comeBack($client, $this->start($client));
        $this->age('codeExpiresAt');
        $this->finish($client, $came);
        self::assertSame('invalid_google_code', $this->body($client)['error'] ?? null, 'code');

        $started = $this->start($client);
        $this->age('expiresAt');
        $client->request('GET', '/v1/auth/google/connect?attempt='.$started['attempt']);
        self::assertNotSame(302, $client->getResponse()->getStatusCode(), 'an expired attempt does not send anybody to Google');
        self::assertSame([], [] === FakeGoogle::$states ? [] : \array_slice(FakeGoogle::$states, 1), 'Google was not given a second state');
    }

    // --- 4. The proof of the device ----------------------------------------------------------------------------------

    public function testWithoutTheProofOfTheDeviceOrWithAnotherDevicesNoTokenIsGiven(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');
        $came = $this->comeBack($client, $this->start($client));
        $elsewhere = sodium_crypto_sign_keypair();

        $cases = [
            ['signature' => $this->encode(random_bytes(64))],
            ['signature' => $this->sign($this->requestChallenge($client)['challenge'], Uuid::v4()->toRfc4122(), sodium_crypto_sign_secretkey($elsewhere))],
            ['challenge' => 'never-issued'],
        ];
        foreach ($cases as $change) {
            $this->finish($client, $change + $came);
            self::assertNotSame(200, $client->getResponse()->getStatusCode());
            self::assertArrayNotHasKey('token', $this->body($client));
        }
        // Not a one of them burned the code: the real device still gets in.
        $this->finish($client, $came);
        self::assertResponseStatusCodeSame(200);
    }

    // --- 5. Google does not confirm the address ----------------------------------------------------------------------

    public function testAnAddressGoogleDoesNotConfirmCreatesAndLinksNothing(): void
    {
        [$client, $google] = $this->world();
        $this->confirmedAccount($client, 'ada@example.com');
        FakeGoogle::$identity = new GoogleIdentity('sub-jhonny', 'ada@example.com', false, 'Jhonny');
        $started = $this->start($client);
        $this->goToGoogle($client, $started);

        $back = $this->returnFromGoogle($client, FakeGoogle::$states[0] ?? '');

        self::assertSame('google_email_not_verified', $back['error'] ?? null);
        self::assertArrayNotHasKey('code', $back);
        self::assertNull($this->user('ada@example.com')->getGoogleSub(), 'not linked');
        FakeGoogle::$identity = new GoogleIdentity('sub-new', 'new@example.com', false, 'New');
        $this->goToGoogle($client, $this->start($client));
        $this->returnFromGoogle($client, FakeGoogle::$states[1] ?? '');
        self::assertNull($this->findUser('new@example.com'), 'nor created');
    }

    // --- 6. The account somebody else prepared -----------------------------------------------------------------------

    public function testAnAccountNobodyConfirmedLosesThePasswordSomeoneElsePutOnIt(): void
    {
        [$client, $google] = $this->world();
        $client->request('POST', '/v1/account/register', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode([
            'email' => 'ada@example.com', 'password' => 'put by Jhonny, never confirmed', 'name' => 'Jhonny', 'locale' => 'en',
        ], \JSON_THROW_ON_ERROR));
        self::assertNotNull($this->user('ada@example.com')->getPassword());
        self::assertFalse($this->user('ada@example.com')->isConfirmed());
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');

        $this->finish($client, $this->comeBack($client, $this->start($client)));

        self::assertResponseStatusCodeSame(200);
        $user = $this->user('ada@example.com');
        self::assertTrue($user->isConfirmed());
        self::assertNull($user->getPassword());
        self::assertSame(0, $this->entityManager()->getRepository(EmailCode::class)->count(['purpose' => EmailCodePurpose::ConfirmEmail]), 'the pending code is gone');
        $this->loginWith($client, 'ada@example.com', 'put by Jhonny, never confirmed');
        self::assertResponseStatusCodeSame(401, 'the password does not enter any more');
    }

    public function testAnAccountThatIsConfirmedIsEnteredAndKeepsItsPassword(): void
    {
        [$client, $google] = $this->world();
        $this->confirmedAccount($client, 'ada@example.com');
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ADA@example.com', true, 'Ada');

        $this->finish($client, $this->comeBack($client, $this->start($client)));

        self::assertResponseStatusCodeSame(200);
        self::assertSame(1, $this->entityManager()->getRepository(User::class)->count([]), 'the same person, the same account');
        self::assertNotNull($this->user('ada@example.com')->getPassword());
        $this->loginWith($client, 'ada@example.com', 'correct horse battery');
        self::assertResponseStatusCodeSame(200);
    }

    // --- 7. The state ------------------------------------------------------------------------------------------------

    public function testTheReturnOfGoogleWithAStateThatIsNotThisBrowsersOrIsRepeatedIsRefused(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');
        $started = $this->start($client);
        $this->goToGoogle($client, $started);
        $state = FakeGoogle::$states[0] ?? '';

        // Another browser: the same state, none of the cookie.
        $client->getCookieJar()->clear();
        $client->request('GET', '/v1/auth/google/callback?code='.FakeGoogle::CODE.'&state='.rawurlencode($state));
        self::assertSame(400, $client->getResponse()->getStatusCode());
        self::assertStringNotContainsString('kosmotheka://', (string) $client->getResponse()->getContent());

        // A state nobody gave.
        $client->request('GET', '/v1/auth/google/callback?code='.FakeGoogle::CODE.'&state='.rawurlencode('made-up'));
        self::assertSame(400, $client->getResponse()->getStatusCode());
        self::assertSame(0, FakeGoogle::$asked, 'Google was not even asked');
    }

    public function testAStateIsGoodOnce(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');
        $this->goToGoogle($client, $this->start($client));
        $state = FakeGoogle::$states[0] ?? '';
        $cookie = $client->getCookieJar()->get('kosmotheka_google', '/v1/auth/google');
        self::assertNotNull($cookie);
        $first = $this->returnFromGoogle($client, $state);
        self::assertArrayHasKey('code', $first);
        // The same browser, with the same cookie, comes again with the same state.
        $client->getCookieJar()->set($cookie);

        $client->request('GET', '/v1/auth/google/callback?code='.FakeGoogle::CODE.'&state='.rawurlencode($state));

        self::assertSame(400, $client->getResponse()->getStatusCode());
        self::assertStringNotContainsString('kosmotheka://', (string) $client->getResponse()->getContent());
        self::assertSame(1, FakeGoogle::$asked);
    }

    public function testAnAttemptSendsToGoogleOnlyOnce(): void
    {
        [$client, $google] = $this->world();
        $started = $this->start($client);
        $this->goToGoogle($client, $started);

        $client->request('GET', '/v1/auth/google/connect?attempt='.$started['attempt']);

        self::assertNotSame(302, $client->getResponse()->getStatusCode());
        self::assertCount(1, FakeGoogle::$states);
    }

    // --- 8. A closed server ------------------------------------------------------------------------------------------

    public function testAServerWithAListOfAddressesMakesNoAccountForOthersWithGoogleEither(): void
    {
        [$client, $google] = $this->world('ada@example.com');
        FakeGoogle::$identity = new GoogleIdentity('sub-jhonny', 'jhonny@example.com', true, 'Jhonny');
        $this->goToGoogle($client, $this->start($client));

        $back = $this->returnFromGoogle($client, FakeGoogle::$states[0] ?? '');

        self::assertSame('registration_closed', $back['error'] ?? null);
        self::assertNull($this->findUser('jhonny@example.com'));

        // One on the list does get in.
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'Ada@example.com', true, 'Ada');
        $this->finish($client, $this->comeBack($client, $this->start($client), 2));
        self::assertResponseStatusCodeSame(200);
    }

    // --- 9. Blocked, second factor -----------------------------------------------------------------------------------

    public function testABlockedAccountDoesNotEnterWithGoogle(): void
    {
        [$client, $google] = $this->world();
        $this->confirmedAccount($client, 'ada@example.com');
        $user = $this->user('ada@example.com');
        $user->block(new \DateTimeImmutable());
        $this->entityManager()->flush();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');
        $this->goToGoogle($client, $this->start($client));

        $back = $this->returnFromGoogle($client, FakeGoogle::$states[0] ?? '');

        self::assertSame('account_blocked', $back['error'] ?? null);
        self::assertArrayNotHasKey('code', $back);
    }

    public function testAnAccountWithASecondFactorAsksForItWithGoogleToo(): void
    {
        [$client, $google] = $this->world();
        $this->confirmedAccount($client, 'ada@example.com');
        $user = $this->user('ada@example.com');
        $secret = 'JBSWY3DPEHPK3PXP';
        $user->setTotpSecret(self::getContainer()->get(TwoFactorSecretBox::class)->seal($user->getId()->toRfc4122(), $secret));
        $user->setTotpEnabledAt(new \DateTimeImmutable());
        $this->entityManager()->flush();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');
        $came = $this->comeBack($client, $this->start($client));

        $this->finish($client, $came);
        self::assertResponseStatusCodeSame(401);
        self::assertSame('two_factor_required', $this->body($client)['error'] ?? null);

        $this->finish($client, ['totp_code' => '000000'] + $came);
        self::assertSame('invalid_two_factor_code', $this->body($client)['error'] ?? null);

        // The code of Google was not burned by asking: with the second factor, the same one gets in.
        $this->finish($client, ['totp_code' => self::totpAt($secret, 'now')] + $came);
        self::assertResponseStatusCodeSame(200);
    }

    // --- 10. A server without Google ---------------------------------------------------------------------------------

    public function testWithoutAGoogleClientNoRouteDoesAnythingAndTheServerDoesNotAnnounceIt(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$configured = false;

        $client->request('GET', '/v1/server');
        self::assertFalse($this->body($client)['sign_in_with_google'] ?? null);

        $client->request('POST', '/v1/auth/google/start', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode(['verifier_hash' => $this->hashOf($this->newVerifier())], \JSON_THROW_ON_ERROR));
        self::assertSame('google_unavailable', $this->body($client)['error'] ?? null);
        $client->request('GET', '/v1/auth/google/connect?attempt=x');
        self::assertSame(404, $client->getResponse()->getStatusCode());
        $client->request('GET', '/v1/auth/google/callback?code=x&state=y');
        self::assertSame(404, $client->getResponse()->getStatusCode());
        $this->finish($client, ['attempt' => 'x', 'code' => 'y', 'verifier' => 'z']);
        self::assertSame('google_unavailable', $this->body($client)['error'] ?? null);
        self::assertSame(0, $this->entityManager()->getRepository(GoogleAttempt::class)->count([]));
        self::assertSame(0, FakeGoogle::$asked + \count(FakeGoogle::$states));
    }

    public function testAServerWithAGoogleClientAnnouncesIt(): void
    {
        [$client] = $this->world();

        $client->request('GET', '/v1/server');

        self::assertTrue($this->body($client)['sign_in_with_google'] ?? null);
    }

    // --- 11. Nothing of it in the log --------------------------------------------------------------------------------

    public function testNeitherTheCodeNorTheVerifierNorAnythingOfGoogleIsKeptInTheLogOrInAnError(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$identity = new GoogleIdentity('sub-secret-1234', 'ada@example.com', true, 'Ada');
        $started = $this->start($client);
        $came = $this->comeBack($client, $started);
        $this->finish($client, ['verifier' => $this->newVerifier()] + $came);
        $wrong = (string) $client->getResponse()->getContent();
        $this->finish($client, $came);

        $text = json_encode(array_map(
            static fn (ActivityEntry $entry): array => (array) $entry,
            $this->entityManager()->getRepository(ActivityEntry::class)->findAll(),
        ), \JSON_PARTIAL_OUTPUT_ON_ERROR) ?: '';
        $rows = $this->entityManager()->getConnection()->fetchAllAssociative('SELECT * FROM activity_log');
        $text .= json_encode($rows, \JSON_THROW_ON_ERROR);
        foreach ([$came['code'], $came['verifier'], 'sub-secret-1234', $started['attempt']] as $secret) {
            self::assertStringNotContainsString($secret, $text);
            self::assertStringNotContainsString($secret, $wrong);
        }
        $kinds = array_column($rows, 'kind');
        self::assertContains('account_created_with_google', $kinds);
        self::assertContains('login_succeeded_with_google', $kinds);
    }

    public function testTheSubOfGoogleIsKeptAsTheAccountsOwnAndNotLogged(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');
        $this->finish($client, $this->comeBack($client, $this->start($client)));

        self::assertSame('sub-ada', $this->user('ada@example.com')->getGoogleSub());
    }

    // --- 12. The same address ----------------------------------------------------------------------------------------

    public function testTwoAddressesThatDifferOnlyInCapitalsAndSpacesAreTheSameAccount(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', ' Ada@EXAMPLE.com ', true, 'Ada');
        $this->finish($client, $this->comeBack($client, $this->start($client)));
        self::assertResponseStatusCodeSame(200);

        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');
        $this->finish($client, $this->comeBack($client, $this->start($client), 2));
        self::assertResponseStatusCodeSame(200);

        self::assertSame(1, $this->entityManager()->getRepository(User::class)->count([]));
    }

    // --- The sub -----------------------------------------------------------------------------------------------------

    public function testAnAccountIsRecognisedByTheSubSoAChangedAddressAtGoogleDoesNotLinkToAnotherAccount(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');
        $this->finish($client, $this->comeBack($client, $this->start($client)));
        $this->confirmedAccount($client, 'hugo@example.com');

        // Ada changes her address at Google to Hugo's: she enters her own account, not Hugo's.
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'hugo@example.com', true, 'Ada');
        $this->finish($client, $this->comeBack($client, $this->start($client), 2));

        self::assertSame('ada@example.com', $this->userBody($client)['email'] ?? null);
        self::assertNull($this->user('hugo@example.com')->getGoogleSub());
    }

    public function testAnotherGoogleAccountWithTheAddressOfALinkedOneDoesNotEnterIt(): void
    {
        [$client, $google] = $this->world();
        FakeGoogle::$identity = new GoogleIdentity('sub-ada', 'ada@example.com', true, 'Ada');
        $this->finish($client, $this->comeBack($client, $this->start($client)));

        FakeGoogle::$identity = new GoogleIdentity('sub-jhonny', 'ada@example.com', true, 'Jhonny');
        $this->goToGoogle($client, $this->start($client));
        $back = $this->returnFromGoogle($client, FakeGoogle::$states[1] ?? '');

        self::assertSame('google_other_account', $back['error'] ?? null);
        self::assertSame('sub-ada', $this->user('ada@example.com')->getGoogleSub());
    }

    // --- Limits, purge, the google that cancels ----------------------------------------------------------------------

    public function testStartingIsLimitedPerAddress(): void
    {
        [$client] = $this->world();

        $answers = [];
        for ($try = 0; $try < 40; ++$try) {
            $client->request('POST', '/v1/auth/google/start', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode(['verifier_hash' => $this->hashOf($this->newVerifier())], \JSON_THROW_ON_ERROR));
            $answers[] = $client->getResponse()->getStatusCode();
        }

        self::assertContains(429, $answers);
    }

    public function testWhoCancelsAtGoogleComesBackWithTheReasonAndNoCode(): void
    {
        [$client, $google] = $this->world();
        $this->goToGoogle($client, $this->start($client));
        $client->request('GET', '/v1/auth/google/callback?error=access_denied&state='.rawurlencode(FakeGoogle::$states[0] ?? ''));

        $back = $this->returnOf($client);

        self::assertSame('google_cancelled', $back['error'] ?? null);
        self::assertArrayNotHasKey('code', $back);
        self::assertSame(0, FakeGoogle::$asked);
    }

    public function testThePurgeDeletesWhatExpired(): void
    {
        [$client] = $this->world();
        $this->start($client);
        $this->start($client);
        $this->age('expiresAt');
        $this->start($client);

        $result = self::getContainer()->get(AccountPurger::class)->purge();

        self::assertSame(2, $result['google_attempts']);
        self::assertSame(1, $this->entityManager()->getRepository(GoogleAttempt::class)->count([]));
    }

    // --- Helpers -----------------------------------------------------------------------------------------------------
    /** @return array{0: KernelBrowser, 1: FakeGoogle} */
    private function world(string $allowed = ''): array
    {
        $_ENV[self::VARIABLE] = $_SERVER[self::VARIABLE] = $allowed;
        self::ensureKernelShutdown();
        $client = self::createClient();
        $client->followRedirects(false);
        self::getContainer()->get('cache.rate_limiter')->clear();
        FakeGoogle::reset();
        FakeGoogle::$configured = true;

        return [$client, new FakeGoogle()];
    }

    private function newVerifier(): string
    {
        return $this->encode(random_bytes(32));
    }

    private function hashOf(string $verifier): string
    {
        return $this->encode(hash('sha256', $verifier, true));
    }

    /** @return array{attempt: string, verifier: string} */
    private function start(KernelBrowser $client, string $locale = 'en'): array
    {
        $verifier = $this->newVerifier();
        $client->request('POST', '/v1/auth/google/start', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode([
            'verifier_hash' => $this->hashOf($verifier),
            'locale' => $locale,
        ], \JSON_THROW_ON_ERROR));
        self::assertResponseStatusCodeSame(200);
        $body = $this->body($client);
        self::assertIsString($body['url'] ?? null);
        self::assertStringStartsWith('http', $body['url']);
        self::assertIsString($body['attempt'] ?? null);

        return ['attempt' => $body['attempt'], 'verifier' => $verifier];
    }

    /** @param array{attempt: string, verifier: string} $started */
    private function goToGoogle(KernelBrowser $client, array $started): void
    {
        $client->request('GET', '/v1/auth/google/connect?attempt='.$started['attempt']);
        self::assertResponseStatusCodeSame(302);
        self::assertStringStartsWith('https://accounts.google.test/', (string) $client->getResponse()->headers->get('Location'));
    }

    /**
     * The browser goes to Google and Google sends it back.
     *
     * @param array{attempt: string, verifier: string} $started
     *
     * @return array{attempt: string, verifier: string, code: string}
     */
    private function comeBack(KernelBrowser $client, array $started, int $nth = 1): array
    {
        $this->goToGoogle($client, $started);
        $back = $this->returnFromGoogle($client, FakeGoogle::$states[$nth - 1] ?? '');
        self::assertArrayHasKey('code', $back, json_encode($back, \JSON_THROW_ON_ERROR));
        self::assertSame($started['attempt'], $back['attempt'] ?? null);

        return ['attempt' => $started['attempt'], 'verifier' => $started['verifier'], 'code' => $back['code']];
    }

    /** @return array<string, string> what the page brought the browser back to the app with */
    private function returnFromGoogle(KernelBrowser $client, string $state): array
    {
        $client->request('GET', '/v1/auth/google/callback?code='.FakeGoogle::CODE.'&state='.rawurlencode($state));

        return $this->returnOf($client);
    }

    /** @return array<string, string> */
    private function returnOf(KernelBrowser $client): array
    {
        $html = (string) $client->getResponse()->getContent();
        if (1 !== preg_match('~kosmotheka://auth/google\?([^"\'<>\s]+)~', html_entity_decode($html), $found)) {
            return ['page' => 'none'];
        }
        parse_str($found[1], $query);
        $values = [];
        foreach ($query as $key => $value) {
            $values[(string) $key] = \is_string($value) ? $value : '';
        }

        return $values;
    }

    /**
     * @param array<string, mixed> $change replaces what the app would send
     * @param array<string, mixed> $came
     *
     * @return array{device_id: string}
     */
    private function finish(KernelBrowser $client, array $change, array $came = []): array
    {
        $deviceId = Uuid::v4()->toRfc4122();
        $keyPair = sodium_crypto_sign_keypair();
        $challenge = $this->requestChallenge($client)['challenge'];
        $body = $change + $came + [
            'device_id' => $deviceId,
            'device_name' => 'Phone',
            'public_key' => $this->encode(sodium_crypto_sign_publickey($keyPair)),
            'challenge' => $challenge,
            'signature' => $this->sign($challenge, $deviceId, sodium_crypto_sign_secretkey($keyPair)),
        ];
        $client->request('POST', '/v1/auth/google/finish', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode($body, \JSON_THROW_ON_ERROR));

        return ['device_id' => $deviceId];
    }

    private function confirmedAccount(KernelBrowser $client, string $email): void
    {
        $this->registerAndConfirmForLogin($client, $email, 'correct horse battery');
        self::getContainer()->get('cache.rate_limiter')->clear();
    }

    private function loginWith(KernelBrowser $client, string $email, string $password): void
    {
        $this->requestLogin($client, $email, $password, Uuid::v4()->toRfc4122(), 'Laptop', $this->encode(random_bytes(32)), 'x', 'y');
        if (Response::HTTP_BAD_REQUEST === $client->getResponse()->getStatusCode()) {
            // The proof is not what is tested here: do it properly.
            $deviceId = Uuid::v4()->toRfc4122();
            $keyPair = sodium_crypto_sign_keypair();
            $challenge = $this->requestChallenge($client)['challenge'];
            $this->requestLogin($client, $email, $password, $deviceId, 'Laptop', $this->encode(sodium_crypto_sign_publickey($keyPair)), $challenge, $this->sign($challenge, $deviceId, sodium_crypto_sign_secretkey($keyPair)));
        }
    }

    /** Makes every attempt (or its code) already past. */
    private function age(string $field): void
    {
        $this->entityManager()->createQuery(\sprintf('UPDATE %s a SET a.%s = :past', GoogleAttempt::class, $field))
            ->setParameter('past', new \DateTimeImmutable('-1 hour'))->execute();
        $this->entityManager()->clear();
    }

    private function user(string $email): User
    {
        return $this->findUser($email) ?? self::fail('No such account: '.$email);
    }

    private function findUser(string $email): ?User
    {
        $this->entityManager()->clear();

        return self::getContainer()->get(UserRepository::class)->findOneByEmail($email);
    }

    /** @return array<string, mixed> */
    private function body(KernelBrowser $client): array
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

    private function tokenOf(KernelBrowser $client): string
    {
        $token = $this->body($client)['token'] ?? '';

        return \is_string($token) ? $token : '';
    }

    /** @return array<string, string> */
    private function userBody(KernelBrowser $client): array
    {
        $user = $this->body($client)['user'] ?? [];
        $fields = [];
        foreach (\is_array($user) ? $user : [] as $key => $value) {
            $fields[(string) $key] = \is_string($value) ? $value : '';
        }

        return $fields;
    }

    private function entityManager(): EntityManagerInterface
    {
        return self::getContainer()->get(EntityManagerInterface::class);
    }
}
