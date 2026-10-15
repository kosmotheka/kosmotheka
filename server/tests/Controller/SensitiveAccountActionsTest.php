<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Entity\Device;
use App\Entity\User;
use App\Enum\EmailCodePurpose;
use App\Repository\EmailCodeRepository;
use App\Tests\Support\DeviceAuthTestTrait;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\Clock\Test\ClockSensitiveTrait;
use Symfony\Component\Mime\Email;
use Symfony\Component\Uid\Uuid;

/**
 * D100: what the account asks for beyond a session when somebody changes how it is entered, and the hostile cases of
 * each. A session lasts long, and a phone that is unlocked in other hands has one.
 */
final class SensitiveAccountActionsTest extends WebTestCase
{
    use ClockSensitiveTrait;
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string AT = '2026-01-02 12:00:00';
    private const string LATER = '2026-01-02 12:00:30';
    private const string PASSWORD = 'correct horse battery';

    private function createTestClient(): KernelBrowser
    {
        $client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();

        return $client;
    }

    // --- 1. The second factor in the delicate actions -------------------------------------------------------------

    public function testChangingThePasswordWithTheSecondFactorOnAsksForItAndChangesNothingWithout(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        [$session] = $this->sessionWithSecondFactor($client, 'change-2fa@example.com');

        $this->send($client, 'POST', '/v1/account/password', $session['headers'], ['current_password' => self::PASSWORD, 'new_password' => 'a brand new password']);
        self::assertResponseStatusCodeSame(401);
        self::assertSame('{"error":"two_factor_required"}', $client->getResponse()->getContent());

        $this->send($client, 'POST', '/v1/account/password', $session['headers'], ['current_password' => self::PASSWORD, 'new_password' => 'a brand new password', 'totp_code' => '000000']);
        self::assertResponseStatusCodeSame(400);
        self::assertSame('{"error":"invalid_two_factor_code"}', $client->getResponse()->getContent());

        $this->assertPasswordIs('change-2fa@example.com', self::PASSWORD);
    }

    public function testChangingThePasswordWithTheRightSecondFactorCodeWorks(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        [$session, $secret] = $this->sessionWithSecondFactor($client, 'change-2fa-ok@example.com');

        self::mockTime(self::LATER);
        $this->send($client, 'POST', '/v1/account/password', $session['headers'], ['current_password' => self::PASSWORD, 'new_password' => 'a brand new password', 'totp_code' => self::totpAt($secret, self::LATER)]);

        self::assertResponseStatusCodeSame(204);
        $this->assertPasswordIs('change-2fa-ok@example.com', 'a brand new password');
    }

    public function testResettingThePasswordWithTheSecondFactorOnAsksForItAndKeepsTheEmailCode(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        [, $secret] = $this->sessionWithSecondFactor($client, 'reset-2fa@example.com');
        $code = $this->codeFromMail(fn () => $this->send($client, 'POST', '/v1/account/password/forgot', [], ['email' => 'reset-2fa@example.com']));

        // The email alone is not enough: that is the point, since the email can be the thing that was lost.
        $this->send($client, 'POST', '/v1/account/password/reset', [], ['email' => 'reset-2fa@example.com', 'code' => $code, 'password' => 'a brand new password']);
        self::assertResponseStatusCodeSame(401);
        self::assertSame('{"error":"two_factor_required"}', $client->getResponse()->getContent());
        $this->send($client, 'POST', '/v1/account/password/reset', [], ['email' => 'reset-2fa@example.com', 'code' => $code, 'password' => 'a brand new password', 'totp_code' => '000000']);
        self::assertResponseStatusCodeSame(400);
        self::assertSame('{"error":"invalid_two_factor_code"}', $client->getResponse()->getContent());
        $this->assertPasswordIs('reset-2fa@example.com', self::PASSWORD);

        // The code of the email was not spent by those: the app asks for the second factor and tries again with it.
        self::mockTime(self::LATER);
        $this->send($client, 'POST', '/v1/account/password/reset', [], ['email' => 'reset-2fa@example.com', 'code' => $code, 'password' => 'a brand new password', 'totp_code' => self::totpAt($secret, self::LATER)]);
        self::assertResponseStatusCodeSame(204);
        $this->assertPasswordIs('reset-2fa@example.com', 'a brand new password');
    }

    public function testNoOneLearnsThatAnAccountHasTheSecondFactorWithoutTheCodeOfItsEmail(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $this->sessionWithSecondFactor($client, 'reset-oracle@example.com');

        $this->send($client, 'POST', '/v1/account/password/reset', [], ['email' => 'reset-oracle@example.com', 'code' => '000000', 'password' => 'a brand new password']);

        self::assertResponseStatusCodeSame(400);
        self::assertSame('{"error":"invalid_code"}', $client->getResponse()->getContent());
    }

    public function testDeletingTheAccountWithTheSecondFactorOnAsksForItAndDeletesNothingWithout(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        [$session] = $this->sessionWithSecondFactor($client, 'delete-2fa@example.com');

        $this->send($client, 'DELETE', '/v1/account', $session['headers'], ['password' => self::PASSWORD]);
        self::assertResponseStatusCodeSame(401);
        self::assertSame('{"error":"two_factor_required"}', $client->getResponse()->getContent());

        $this->send($client, 'DELETE', '/v1/account', $session['headers'], ['password' => self::PASSWORD, 'totp_code' => '000000']);
        self::assertResponseStatusCodeSame(400);
        self::assertSame('{"error":"invalid_two_factor_code"}', $client->getResponse()->getContent());

        self::assertNotNull($this->findUser('delete-2fa@example.com'));
    }

    // --- 2. Turning the second factor on asks for the password ----------------------------------------------------

    public function testTheSessionAloneDoesNotTurnTheSecondFactorOn(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'enable-session@example.com');
        $secret = $this->startSetup($client, $session['headers']);

        $this->send($client, 'POST', '/v1/account/two-factor/enable', $session['headers'], ['code' => self::totpAt($secret, self::AT)]);
        self::assertResponseStatusCodeSame(401);
        self::assertSame('{"error":"invalid_credentials"}', $client->getResponse()->getContent());

        $this->send($client, 'POST', '/v1/account/two-factor/enable', $session['headers'], ['code' => self::totpAt($secret, self::AT), 'password' => 'not the password']);
        self::assertResponseStatusCodeSame(401);
        self::assertSame('{"error":"invalid_credentials"}', $client->getResponse()->getContent());

        $this->entityManager()->clear();
        $user = $this->findUser('enable-session@example.com');
        self::assertNotNull($user);
        self::assertFalse($user->isTwoFactorEnabled());
        self::assertNotNull($user->getTotpPendingSecret(), 'The pending secret is still there, waiting.');

        $this->send($client, 'POST', '/v1/account/two-factor/enable', $session['headers'], ['code' => self::totpAt($secret, self::AT), 'password' => self::PASSWORD]);
        self::assertResponseStatusCodeSame(200);
    }

    public function testGuessingThePasswordToTurnTheSecondFactorOnIsCutPerAccountLikeSigningIn(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'enable-guess@example.com');
        $secret = $this->startSetup($client, $session['headers']);

        for ($i = 0; $i < 5; ++$i) {
            $this->send($client, 'POST', '/v1/account/two-factor/enable', $session['headers'], ['code' => self::totpAt($secret, self::AT), 'password' => 'guess '.$i]);
            self::assertResponseStatusCodeSame(401);
        }

        $this->send($client, 'POST', '/v1/account/two-factor/enable', $session['headers'], ['code' => self::totpAt($secret, self::AT), 'password' => self::PASSWORD]);
        self::assertResponseStatusCodeSame(429);
    }

    public function testAnEmailCodeDoesNotReplaceThePasswordOfAnAccountThatHasOne(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'enable-code-not-password@example.com');
        $secret = $this->startSetup($client, $session['headers']);
        $emailCode = $this->actionCode($client, $session['headers']);

        $this->send($client, 'POST', '/v1/account/two-factor/enable', $session['headers'], ['code' => self::totpAt($secret, self::AT), 'email_code' => $emailCode]);

        self::assertResponseStatusCodeSame(401);
        $this->entityManager()->clear();
        $user = $this->findUser('enable-code-not-password@example.com');
        self::assertNotNull($user);
        self::assertFalse($user->isTwoFactorEnabled());
    }

    // --- 3. An account with no password (only Google) -------------------------------------------------------------

    public function testAnAccountWithNoPasswordTurnsTheSecondFactorOnWithACodeFromItsEmail(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $headers = $this->passwordlessSession('google-enable@example.com');
        $secret = $this->startSetup($client, $headers);

        $this->send($client, 'POST', '/v1/account/two-factor/enable', $headers, ['code' => self::totpAt($secret, self::AT)]);
        self::assertResponseStatusCodeSame(401, 'No password and no code from the email: the session alone is not enough.');

        $emailCode = $this->actionCode($client, $headers);
        $this->send($client, 'POST', '/v1/account/two-factor/enable', $headers, ['code' => self::totpAt($secret, self::AT), 'email_code' => $emailCode]);

        self::assertResponseStatusCodeSame(200);
        $this->entityManager()->clear();
        $user = $this->findUser('google-enable@example.com');
        self::assertNotNull($user);
        self::assertTrue($user->isTwoFactorEnabled());
    }

    public function testAWrongSecondFactorCodeDoesNotSpendTheEmailCodeOfTheAction(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $headers = $this->passwordlessSession('google-enable-retry@example.com');
        $secret = $this->startSetup($client, $headers);
        $emailCode = $this->actionCode($client, $headers);

        $this->send($client, 'POST', '/v1/account/two-factor/enable', $headers, ['code' => '000000', 'email_code' => $emailCode]);
        self::assertResponseStatusCodeSame(400);

        $this->send($client, 'POST', '/v1/account/two-factor/enable', $headers, ['code' => self::totpAt($secret, self::AT), 'email_code' => $emailCode]);
        self::assertResponseStatusCodeSame(200);
    }

    public function testAnAccountWithNoPasswordDeletesItselfWithACodeFromItsEmailAndNotWithout(): void
    {
        $client = $this->createTestClient();
        $headers = $this->passwordlessSession('google-delete@example.com');

        $this->send($client, 'DELETE', '/v1/account', $headers, []);
        self::assertResponseStatusCodeSame(401);
        $this->send($client, 'DELETE', '/v1/account', $headers, ['email_code' => '000000']);
        self::assertResponseStatusCodeSame(400);
        $this->send($client, 'DELETE', '/v1/account', $headers, ['password' => 'anything at all']);
        self::assertResponseStatusCodeSame(401, 'It has no password: a guess is not a way in.');
        self::assertNotNull($this->findUser('google-delete@example.com'));

        $emailCode = $this->actionCode($client, $headers);
        $this->send($client, 'DELETE', '/v1/account', $headers, ['email_code' => $emailCode]);

        self::assertResponseStatusCodeSame(204);
        self::assertNull($this->findUser('google-delete@example.com'));
    }

    public function testAnAccountWithNoPasswordAndTheSecondFactorOnAsksForBothToBeDeleted(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $headers = $this->passwordlessSession('google-delete-2fa@example.com');
        $secret = $this->enableForPasswordless($client, $headers);

        $emailCode = $this->actionCode($client, $headers);
        $this->send($client, 'DELETE', '/v1/account', $headers, ['email_code' => $emailCode]);
        self::assertResponseStatusCodeSame(401);
        self::assertSame('{"error":"two_factor_required"}', $client->getResponse()->getContent());
        self::assertNotNull($this->findUser('google-delete-2fa@example.com'));

        self::mockTime(self::LATER);
        $this->send($client, 'DELETE', '/v1/account', $headers, ['email_code' => $emailCode, 'totp_code' => self::totpAt($secret, self::LATER)]);
        self::assertResponseStatusCodeSame(204);
    }

    public function testAnAccountWithNoPasswordTakesTheSecondFactorOffAndRegeneratesItsCodesWithTheSecondFactorAlone(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $headers = $this->passwordlessSession('google-off@example.com');
        $secret = $this->enableForPasswordless($client, $headers);

        self::mockTime(self::LATER);
        $this->send($client, 'POST', '/v1/account/two-factor/backup-codes', $headers, ['code' => self::totpAt($secret, self::LATER)]);
        self::assertResponseStatusCodeSame(200);

        $this->send($client, 'DELETE', '/v1/account/two-factor', $headers, ['code' => '000000']);
        self::assertResponseStatusCodeSame(400, 'A wrong code still does not take it off.');

        self::mockTime('2026-01-02 12:01:00');
        $this->send($client, 'DELETE', '/v1/account/two-factor', $headers, ['code' => self::totpAt($secret, '2026-01-02 12:01:00')]);
        self::assertResponseStatusCodeSame(204);
        $this->entityManager()->clear();
        $user = $this->findUser('google-off@example.com');
        self::assertNotNull($user);
        self::assertFalse($user->isTwoFactorEnabled());
    }

    // --- 4. A code for one thing is no use for another ------------------------------------------------------------

    public function testTheCodeOfAnActionDoesNotResetAPassword(): void
    {
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'cross-action@example.com');
        $actionCode = $this->actionCode($client, $session['headers']);

        $this->send($client, 'POST', '/v1/account/password/reset', [], ['email' => 'cross-action@example.com', 'code' => $actionCode, 'password' => 'a brand new password']);

        self::assertResponseStatusCodeSame(400);
        self::assertSame('{"error":"invalid_code"}', $client->getResponse()->getContent());
        $this->assertPasswordIs('cross-action@example.com', self::PASSWORD);
    }

    public function testTheCodeToResetAPasswordDoesNotDeleteAnAccount(): void
    {
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'cross-reset@example.com');
        $resetCode = $this->codeFromMail(fn () => $this->send($client, 'POST', '/v1/account/password/forgot', [], ['email' => 'cross-reset@example.com']));
        $user = $this->findUser('cross-reset@example.com');
        self::assertNotNull($user);
        $user->clearPassword();
        $this->entityManager()->flush();

        $this->send($client, 'DELETE', '/v1/account', $session['headers'], ['email_code' => $resetCode]);

        self::assertResponseStatusCodeSame(400);
        self::assertNotNull($this->findUser('cross-reset@example.com'));
    }

    public function testACodeForAnActionIsOfItsOwnPurposeAndIsOnlyAskedForWithASession(): void
    {
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'cross-confirm@example.com');
        $this->actionCode($client, $session['headers']);
        $user = $this->findUser('cross-confirm@example.com');
        self::assertNotNull($user);
        $emailCodes = self::getContainer()->get(EmailCodeRepository::class);

        self::assertNotNull($emailCodes->findOneByUserAndPurpose($user, EmailCodePurpose::ConfirmAction));
        self::assertNull($emailCodes->findOneByUserAndPurpose($user, EmailCodePurpose::ConfirmEmail));

        $this->send($client, 'POST', '/v1/account/code/action', [], []);
        self::assertResponseStatusCodeSame(401);
    }

    // --- 6. The answers do not tell whether an address has an account ---------------------------------------------

    public function testTheFourthCodeAskedForAnAddressWithAnAccountIsAnsweredAsForOneWithout(): void
    {
        $client = $this->createTestClient();
        $this->send($client, 'POST', '/v1/account/register', [], ['email' => 'has-account@example.com', 'password' => self::PASSWORD, 'name' => 'Ada', 'locale' => 'en']);

        $answers = [];
        foreach (['has-account@example.com', 'no-account@example.com'] as $email) {
            for ($i = 0; $i < 4; ++$i) {
                $this->send($client, 'POST', '/v1/account/code', [], ['email' => $email, 'purpose' => 'confirm_email']);
                $answers[$email][] = [$client->getResponse()->getStatusCode(), $client->getResponse()->getContent()];
            }
        }

        self::assertSame($answers['no-account@example.com'], $answers['has-account@example.com']);
        self::assertSame(202, $answers['has-account@example.com'][3][0]);
    }

    public function testTheFourthPasswordResetAskedForAnAddressWithAnAccountIsAnsweredAsForOneWithout(): void
    {
        $client = $this->createTestClient();
        $this->signInNewDevice($client, 'forgot-has@example.com');

        $answers = [];
        foreach (['forgot-has@example.com', 'forgot-none@example.com'] as $email) {
            for ($i = 0; $i < 4; ++$i) {
                $this->send($client, 'POST', '/v1/account/password/forgot', [], ['email' => $email]);
                $answers[$email][] = [$client->getResponse()->getStatusCode(), $client->getResponse()->getContent()];
            }
        }

        self::assertSame($answers['forgot-none@example.com'], $answers['forgot-has@example.com']);
        self::assertSame(202, $answers['forgot-has@example.com'][3][0]);
    }

    public function testRegisteringAnAddressWhoseMailsAreSpentIsAnsweredAsAlwaysAndSendsNothing(): void
    {
        $client = $this->createTestClient();
        $this->signInNewDevice($client, 'spent@example.com');
        for ($i = 0; $i < 3; ++$i) {
            $this->send($client, 'POST', '/v1/account/password/forgot', [], ['email' => 'spent@example.com']);
        }

        $this->send($client, 'POST', '/v1/account/register', [], ['email' => 'spent@example.com', 'password' => self::PASSWORD, 'name' => 'Ada', 'locale' => 'en']);

        self::assertResponseStatusCodeSame(202);
        self::assertSame('{}', $client->getResponse()->getContent());
        self::assertEmailCount(0);
    }

    // --- 8. Registering again an address nobody has confirmed -----------------------------------------------------

    public function testRegisteringAgainAnUnconfirmedAddressDoesNotChangeThePasswordAndTheOneGivenWhenConfirmingIsTheOneThatStays(): void
    {
        $client = $this->createTestClient();
        $email = 'ana@example.com';
        $this->send($client, 'POST', '/v1/account/register', [], ['email' => $email, 'password' => 'the password of ana', 'name' => 'Ana', 'locale' => 'en']);
        $before = $this->findUser($email)?->getPassword();

        // Whoever registers after Ana makes Ana's code die: the code that arrives is the second one.
        $code = $this->codeFromMail(fn () => $this->send($client, 'POST', '/v1/account/register', [], ['email' => $email, 'password' => 'the password of the intruder', 'name' => 'Ana', 'locale' => 'en']));
        $this->entityManager()->clear();
        self::assertSame($before, $this->findUser($email)?->getPassword(), 'Registering again changed the password.');

        $this->send($client, 'POST', '/v1/account/confirm', [], ['email' => $email, 'code' => $code, 'password' => 'the password of ana']);
        self::assertResponseStatusCodeSame(204);

        $this->entityManager()->clear();
        $this->assertPasswordIs($email, 'the password of ana');
        $this->assertPasswordIsNot($email, 'the password of the intruder');
    }

    public function testRegisteringAgainAnUnconfirmedAddressChangesNothingButSendsANewCode(): void
    {
        $client = $this->createTestClient();
        $email = 'ana-again@example.com';
        $this->send($client, 'POST', '/v1/account/register', [], ['email' => $email, 'password' => 'the password of ana', 'name' => 'Ana', 'locale' => 'en']);
        $before = $this->findUser($email);
        self::assertNotNull($before);
        $passwordBefore = $before->getPassword();

        $code = $this->codeFromMail(fn () => $this->send($client, 'POST', '/v1/account/register', [], ['email' => $email, 'password' => 'the password of the intruder', 'name' => 'Evil Name', 'locale' => 'es']));

        self::assertNotSame('', $code);
        $after = $this->findUser($email);
        self::assertNotNull($after);
        self::assertSame('Ana', $after->getName(), 'Registering again changed the name.');
        self::assertSame('en', $after->getLocale(), 'Registering again changed the language.');
        self::assertSame($passwordBefore, $after->getPassword());
    }

    public function testConfirmingNeedsAPassword(): void
    {
        $client = $this->createTestClient();
        $email = 'no-password-at-confirm@example.com';
        $code = $this->codeFromMail(fn () => $this->send($client, 'POST', '/v1/account/register', [], ['email' => $email, 'password' => self::PASSWORD, 'name' => 'Ada', 'locale' => 'en']));

        $this->send($client, 'POST', '/v1/account/confirm', [], ['email' => $email, 'code' => $code]);
        self::assertResponseStatusCodeSame(422);
        $this->send($client, 'POST', '/v1/account/confirm', [], ['email' => $email, 'code' => $code, 'password' => 'short']);
        self::assertResponseStatusCodeSame(422);

        $this->entityManager()->clear();
        self::assertFalse($this->findUser($email)?->isConfirmed());
    }

    // --- Helpers ----------------------------------------------------------------------------------------------------

    /**
     * @param array<string, string> $headers
     * @param array<string, mixed>  $payload
     */
    private function send(KernelBrowser $client, string $method, string $uri, array $headers, array $payload): void
    {
        $client->request($method, $uri, server: [...$headers, 'CONTENT_TYPE' => 'application/json'], content: json_encode($payload, \JSON_THROW_ON_ERROR));
    }

    /**
     * @return array{0: array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string}, 1: string}
     */
    private function sessionWithSecondFactor(KernelBrowser $client, string $email): array
    {
        $session = $this->signInNewDevice($client, $email);
        $secret = $this->startSetup($client, $session['headers']);
        $this->send($client, 'POST', '/v1/account/two-factor/enable', $session['headers'], ['code' => self::totpAt($secret, self::AT), 'password' => self::PASSWORD]);
        self::assertResponseStatusCodeSame(200);

        return [$session, $secret];
    }

    /** @param array<string, string> $headers */
    private function startSetup(KernelBrowser $client, array $headers): string
    {
        $client->request('POST', '/v1/account/two-factor/setup', server: $headers);
        /** @var array{secret: string} $body */
        $body = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);

        return $body['secret'];
    }

    /** @param array<string, string> $headers */
    private function enableForPasswordless(KernelBrowser $client, array $headers): string
    {
        $secret = $this->startSetup($client, $headers);
        $emailCode = $this->actionCode($client, $headers);
        $this->send($client, 'POST', '/v1/account/two-factor/enable', $headers, ['code' => self::totpAt($secret, self::AT), 'email_code' => $emailCode]);
        self::assertResponseStatusCodeSame(200);

        return $secret;
    }

    /**
     * The code of the email that a signed-in account asks for to do something delicate.
     *
     * @param array<string, string> $headers
     */
    private function actionCode(KernelBrowser $client, array $headers): string
    {
        $code = $this->codeFromMail(fn () => $this->send($client, 'POST', '/v1/account/code/action', $headers, []));
        self::assertResponseStatusCodeSame(202);

        return $code;
    }

    private function codeFromMail(callable $ask): string
    {
        $ask();
        $message = self::getMailerMessage();
        self::assertInstanceOf(Email::class, $message, 'An email was expected.');
        preg_match('/\d{6}/', (string) $message->getTextBody(), $matches);

        return $matches[0] ?? self::fail('No code in the email.');
    }

    /**
     * A confirmed account with no password, as one made with Google is, with a session of its own.
     *
     * @return array{HTTP_X-Device-Id: string, HTTP_Authorization: string}
     */
    private function passwordlessSession(string $email): array
    {
        $entityManager = $this->entityManager();
        $user = new User($email, 'Ada', 'en');
        $user->confirm(new \DateTimeImmutable());
        $entityManager->persist($user);
        $deviceId = Uuid::v4()->toRfc4122();
        $token = bin2hex(random_bytes(16));
        $entityManager->persist(new Device($deviceId, $user, 'Test device', sodium_bin2base64(random_bytes(32), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING), hash('sha256', $token)));
        $entityManager->flush();

        return ['HTTP_X-Device-Id' => $deviceId, 'HTTP_Authorization' => 'Bearer '.$token];
    }

    private function assertPasswordIs(string $email, string $password): void
    {
        $user = $this->findUser($email);
        self::assertNotNull($user);
        $hash = $user->getPassword();
        self::assertNotNull($hash);
        self::assertTrue(self::getContainer()->get('security.user_password_hasher')->isPasswordValid($user, $password), 'The password of '.$email.' is not the one expected.');
    }

    private function assertPasswordIsNot(string $email, string $password): void
    {
        $user = $this->findUser($email);
        self::assertNotNull($user);
        self::assertFalse(self::getContainer()->get('security.user_password_hasher')->isPasswordValid($user, $password));
    }

    private function findUser(string $email): ?User
    {
        // What the requests wrote went through another entity manager: read it again, not from this one's memory.
        $this->entityManager()->clear();

        return $this->entityManager()->getRepository(User::class)->findOneBy(['email' => $email]);
    }

    private function entityManager(): EntityManagerInterface
    {
        return self::getContainer()->get(EntityManagerInterface::class);
    }
}
