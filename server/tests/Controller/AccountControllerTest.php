<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Entity\Device;
use App\Entity\User;
use App\Enum\EmailCodePurpose;
use App\Repository\DeviceRepository;
use App\Repository\EmailCodeRepository;
use App\Sync\Invitation\EmailFingerprint;
use App\Tests\Support\DeviceAuthTestTrait;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\Mime\Email;
use Symfony\Component\Uid\Uuid;

/**
 * Functional coverage of `/v1/account/*` (docs/architecture/accounts.md):
 * registration never reveals whether an email exists, codes expire, allow 5
 * wrong attempts, and are single use; plus the authenticated account and
 * device endpoints added in milestone 7.1, part 2.
 */
final class AccountControllerTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    /**
     * WebTestCase forbids booting the kernel before createClient(), so the
     * rate limiter cache (filesystem, not rolled back by dama/doctrine-test-
     * bundle like the database) is cleared here, right after, instead of in
     * setUp(). Every test starts from this instead of createClient() directly.
     */
    private function createTestClient(): KernelBrowser
    {
        $client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();

        return $client;
    }

    public function testRegisterNewEmailSendsAConfirmationCode(): void
    {
        $client = $this->createTestClient();

        $this->register($client, 'new@example.com', 'Ada', 'en');

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertSame('{}', $client->getResponse()->getContent());
        self::assertEmailCount(1);

        $message = self::getMailerMessage();
        self::assertInstanceOf(Email::class, $message);
        self::assertStringContainsString('new@example.com', $message->getTo()[0]->getAddress());

        $user = $this->findUser('new@example.com');
        self::assertNotNull($user);
        self::assertFalse($user->isConfirmed());
    }

    public function testRegisterExistingUnconfirmedUserChangesNothingAndResendsACode(): void
    {
        $client = $this->createTestClient();
        $this->register($client, 'pending@example.com', 'Old Name', 'en');

        $this->register($client, 'pending@example.com', 'New Name', 'es', 'another good password');

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertEmailCount(1);

        $user = $this->findUser('pending@example.com');
        self::assertNotNull($user);
        self::assertSame('Old Name', $user->getName());
        self::assertSame('en', $user->getLocale());
    }

    public function testRegisterExistingConfirmedUserChangesNothingAndSendsANotice(): void
    {
        $client = $this->createTestClient();
        $email = $this->registerAndConfirm($client, 'confirmed@example.com');

        $this->register($client, $email, 'Someone else', 'en', 'a different password');

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertEmailCount(1);

        $user = $this->findUser($email);
        self::assertNotNull($user);
        self::assertSame('Ada', $user->getName());
    }

    public function testRegisterInSpanishSendsTheEmailInSpanish(): void
    {
        $client = $this->createTestClient();

        $this->register($client, 'es-speaker@example.com', 'Ada', 'es');

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);

        $message = self::getMailerMessage();
        self::assertInstanceOf(Email::class, $message);
        self::assertStringContainsString('Confirma tu cuenta', $message->getSubject() ?? '');
    }

    /**
     * @param array<string, mixed> $payload
     */
    #[DataProvider('invalidRegistrationPayloads')]
    public function testRegisterRejectsInvalidPayloads(array $payload): void
    {
        $client = $this->createTestClient();

        $client->request('POST', '/v1/account/register', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode($payload, \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_UNPROCESSABLE_ENTITY);
    }

    /**
     * @return iterable<string, array{array<string, mixed>}>
     */
    public static function invalidRegistrationPayloads(): iterable
    {
        $valid = ['email' => 'valid@example.com', 'password' => 'a long enough password', 'name' => 'Ada', 'locale' => 'en'];

        yield 'invalid email' => [[...$valid, 'email' => 'not-an-email']];
        yield 'short password' => [[...$valid, 'password' => 'short']];
        yield 'blank name' => [[...$valid, 'name' => ' ']];
        yield 'unknown locale' => [[...$valid, 'locale' => 'fr']];
    }

    public function testRegisterIsRateLimitedPerIp(): void
    {
        $client = $this->createTestClient();

        for ($i = 0; $i < 5; ++$i) {
            $this->register($client, \sprintf('ip-limit-%d@example.com', $i), 'Ada', 'en');
            self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        }

        $this->register($client, 'ip-limit-5@example.com', 'Ada', 'en');

        self::assertResponseStatusCodeSame(Response::HTTP_TOO_MANY_REQUESTS);
        self::assertSame('{"error":"too_many_requests"}', $client->getResponse()->getContent());
        self::assertTrue($client->getResponse()->headers->has('Retry-After'));
    }

    public function testConfirmWithTheRightCodeConfirmsTheAccount(): void
    {
        $client = $this->createTestClient();
        $email = 'confirm-me@example.com';
        $code = $this->registerAndGetCode($client, $email);

        $this->confirm($client, $email, $code);

        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        $user = $this->findUser($email);
        self::assertNotNull($user);
        self::assertTrue($user->isConfirmed());
    }

    public function testConfirmWithAWrongCodeFails(): void
    {
        $client = $this->createTestClient();
        $email = 'wrong-code@example.com';
        $this->registerAndGetCode($client, $email);

        $this->confirm($client, $email, '000000');

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_code"}', $client->getResponse()->getContent());
    }

    public function testConfirmWithAnExpiredCodeFails(): void
    {
        $client = $this->createTestClient();
        $email = 'expired-code@example.com';
        $code = $this->registerAndGetCode($client, $email);

        $user = $this->findUser($email);
        self::assertNotNull($user);
        $emailCode = self::getContainer()->get(EmailCodeRepository::class)->findOneByUserAndPurpose($user, EmailCodePurpose::ConfirmEmail);
        self::assertNotNull($emailCode);
        new \ReflectionProperty($emailCode, 'expiresAt')->setValue($emailCode, new \DateTimeImmutable('-1 minute'));
        $this->entityManager()->flush();

        $this->confirm($client, $email, $code);

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"code_expired"}', $client->getResponse()->getContent());
    }

    public function testConfirmLocksAfterFiveWrongAttempts(): void
    {
        $client = $this->createTestClient();
        $email = 'five-attempts@example.com';
        $this->registerAndGetCode($client, $email);

        for ($i = 0; $i < 4; ++$i) {
            $this->confirm($client, $email, '000000');
            self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
            self::assertSame('{"error":"invalid_code"}', $client->getResponse()->getContent());
        }

        $this->confirm($client, $email, '000000');

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"too_many_attempts"}', $client->getResponse()->getContent());
    }

    public function testConfirmCannotReuseAUsedCode(): void
    {
        $client = $this->createTestClient();
        $email = 'reuse@example.com';
        $code = $this->registerAndGetCode($client, $email);

        $this->confirm($client, $email, $code);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        $this->confirm($client, $email, $code);
        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_code"}', $client->getResponse()->getContent());
    }

    public function testConfirmOfAnAlreadyConfirmedAccountFails(): void
    {
        $client = $this->createTestClient();
        $email = $this->registerAndConfirm($client, 'already-confirmed@example.com');

        $this->confirm($client, $email, '123456');

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_code"}', $client->getResponse()->getContent());
    }

    public function testConfirmOfAnUnknownEmailFails(): void
    {
        $client = $this->createTestClient();

        $this->confirm($client, 'nobody@example.com', '123456');

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_code"}', $client->getResponse()->getContent());
    }

    public function testRequestCodeResendsOnlyForAnUnconfirmedUser(): void
    {
        $client = $this->createTestClient();
        $this->register($client, 'resend@example.com', 'Ada', 'en');

        $this->requestCode($client, 'resend@example.com');

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertEmailCount(1);
    }

    public function testRequestCodeForAConfirmedUserSendsNothing(): void
    {
        $client = $this->createTestClient();
        $email = $this->registerAndConfirm($client, 'already-done@example.com');

        $this->requestCode($client, $email);

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertEmailCount(0);
    }

    public function testRequestCodeForAnUnknownEmailSendsNothing(): void
    {
        $client = $this->createTestClient();

        $this->requestCode($client, 'unknown@example.com');

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertEmailCount(0);
    }

    public function testRequestCodeRejectsAnUnsupportedPurpose(): void
    {
        $client = $this->createTestClient();

        $client->request('POST', '/v1/account/code', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode(['email' => 'someone@example.com', 'purpose' => 'reset_password'], \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_UNPROCESSABLE_ENTITY);
    }

    public function testACodeAskedPastTheLimitOfTheAddressLeavesTheLastCodeSentWorking(): void
    {
        $client = $this->createTestClient();
        $email = 'kept-code@example.com';
        $this->register($client, $email, 'Ada', 'en');
        $this->requestCode($client, $email);
        $this->requestCode($client, $email);
        $message = self::getMailerMessage();
        self::assertInstanceOf(Email::class, $message);
        preg_match('/\d{6}/', (string) $message->getTextBody(), $matches);
        $lastCode = $matches[0] ?? self::fail('No code in the email.');

        // Anyone may ask for a code for this address; once its mails are spent, the code it already has must stay,
        // and the answer is the same as ever (D100).
        $this->requestCode($client, $email);
        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertEmailCount(0);

        $this->confirm($client, $email, $lastCode);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);
    }

    public function testTheMailLimitOfAnAddressDoesNotKeepTheAddress(): void
    {
        $client = $this->createTestClient();
        $email = 'not-in-the-cache@example.com';

        $this->register($client, $email, 'Ada', 'en');
        self::assertEmailCount(1);

        $pool = self::getContainer()->get('cache.rate_limiter');
        $fingerprint = self::getContainer()->get(EmailFingerprint::class)->of($email);
        self::assertFalse($pool->hasItem(sha1('email_per_address-'.$email)));
        self::assertTrue($pool->hasItem(sha1('email_per_address-'.$fingerprint)));
    }

    private function register(KernelBrowser $client, string $email, string $name, string $locale, string $password = 'correct horse battery'): void
    {
        $client->request('POST', '/v1/account/register', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode([
            'email' => $email,
            'password' => $password,
            'name' => $name,
            'locale' => $locale,
        ], \JSON_THROW_ON_ERROR));
    }

    private function confirm(KernelBrowser $client, string $email, string $code): void
    {
        $client->request('POST', '/v1/account/confirm', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode(['email' => $email, 'code' => $code, 'password' => 'correct horse battery'], \JSON_THROW_ON_ERROR));
    }

    private function requestCode(KernelBrowser $client, string $email, string $purpose = 'confirm_email'): void
    {
        $client->request('POST', '/v1/account/code', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode(['email' => $email, 'purpose' => $purpose], \JSON_THROW_ON_ERROR));
    }

    private function registerAndGetCode(KernelBrowser $client, string $email): string
    {
        $this->register($client, $email, 'Ada', 'en');

        $message = self::getMailerMessage();
        self::assertInstanceOf(Email::class, $message);
        $body = (string) $message->getTextBody();
        self::assertMatchesRegularExpression('/\d{6}/', $body, 'The email should contain a 6-digit code.');
        preg_match('/\d{6}/', $body, $matches);

        return $matches[0] ?? self::fail('No code in the email.');
    }

    private function registerAndConfirm(KernelBrowser $client, string $email): string
    {
        $code = $this->registerAndGetCode($client, $email);
        $this->confirm($client, $email, $code);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        return $email;
    }

    public function testShowReturnsTheAccount(): void
    {
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'show@example.com');

        $client->request('GET', '/v1/account', server: $session['headers']);

        self::assertResponseStatusCodeSame(Response::HTTP_OK);
        /** @var array<string, mixed> $body */
        $body = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);
        self::assertSame('show@example.com', $body['email']);
        self::assertSame('Ada', $body['name']);
        self::assertSame('en', $body['locale']);
        self::assertTrue($body['has_password']);
    }

    public function testUpdateChangesNameAndLocale(): void
    {
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'update@example.com');

        $client->request('PATCH', '/v1/account', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode(['name' => 'New Name', 'locale' => 'es'], \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_OK);
        /** @var array<string, mixed> $body */
        $body = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);
        self::assertSame('New Name', $body['name']);
        self::assertSame('es', $body['locale']);

        $user = $this->findUser('update@example.com');
        self::assertNotNull($user);
        self::assertSame('New Name', $user->getName());
        self::assertSame('es', $user->getLocale());
    }

    public function testUpdateWithoutFieldsChangesNothing(): void
    {
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'update-nothing@example.com');

        $client->request('PATCH', '/v1/account', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: '{}');

        self::assertResponseStatusCodeSame(Response::HTTP_OK);
        /** @var array<string, mixed> $body */
        $body = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);
        self::assertSame('Ada', $body['name']);
        self::assertSame('en', $body['locale']);
    }

    public function testListDevicesMarksTheCurrentOne(): void
    {
        $client = $this->createTestClient();
        $email = 'devices-list@example.com';
        $first = $this->signInNewDevice($client, $email, deviceName: 'Laptop');
        $second = $this->loginDevice($client, $email, 'correct horse battery', Uuid::v4()->toRfc4122(), 'Phone');

        $client->request('GET', '/v1/account/devices', server: $second['headers']);

        self::assertResponseStatusCodeSame(Response::HTTP_OK);
        /** @var list<array{id: string, name: string, signed_in: bool, current: bool}> $devices */
        $devices = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);
        self::assertCount(2, $devices);
        $byId = array_combine(array_column($devices, 'id'), $devices);
        self::assertFalse($byId[$first['device_id']]['current']);
        self::assertTrue($byId[$second['device_id']]['current']);
        self::assertTrue($byId[$first['device_id']]['signed_in']);
    }

    public function testRemovingAnotherOwnDeviceSignsItOutForGood(): void
    {
        $client = $this->createTestClient();
        $email = 'revoke-own@example.com';
        $first = $this->signInNewDevice($client, $email, deviceName: 'Laptop');
        $second = $this->loginDevice($client, $email, 'correct horse battery', Uuid::v4()->toRfc4122(), 'Phone');

        $client->request('DELETE', '/v1/account/devices/'.$first['device_id'], server: $second['headers']);

        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        $client->request('GET', '/v1/account', server: $first['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"device_removed"}', $client->getResponse()->getContent());

        $device = self::getContainer()->get(DeviceRepository::class)->findOneBy(['deviceId' => $first['device_id']]);
        self::assertInstanceOf(Device::class, $device);
        self::assertFalse($device->isSignedIn());
        self::assertTrue($device->isRemoved());

        $client->request('GET', '/v1/account/devices', server: $second['headers']);
        $body = $client->getResponse()->getContent();
        self::assertIsString($body);
        $devices = json_decode($body, true, flags: \JSON_THROW_ON_ERROR);
        self::assertIsArray($devices);
        $removed = array_column($devices, 'removed', 'id');
        self::assertTrue($removed[$first['device_id']] ?? null);
        self::assertFalse($removed[$second['device_id']] ?? null);
    }

    public function testRemovingTheCallingDeviceLeavesItToSomeoneElse(): void
    {
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'leave-device@example.com');

        $client->request('DELETE', '/v1/account/devices/'.$session['device_id'], server: $session['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        $client->request('GET', '/v1/account', server: $session['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"device_removed"}', $client->getResponse()->getContent());
    }

    public function testARemovedDeviceCannotSignInAgainWithAnyAccount(): void
    {
        $client = $this->createTestClient();
        $owner = $this->signInNewDevice($client, 'removed-owner@example.com');
        $client->request('DELETE', '/v1/account/devices/'.$owner['device_id'], server: $owner['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        $this->registerAndConfirmForLogin($client, 'removed-next@example.com', 'correct horse battery');

        foreach (['removed-owner@example.com', 'removed-next@example.com'] as $email) {
            $challenge = $this->requestChallenge($client)['challenge'];
            $signature = $this->sign($challenge, $owner['device_id'], sodium_base642bin($owner['secret_key'], \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING));
            $this->requestLogin($client, $email, 'correct horse battery', $owner['device_id'], 'Phone', $owner['public_key'], $challenge, $signature);

            self::assertResponseStatusCodeSame(Response::HTTP_FORBIDDEN);
            self::assertSame('{"error":"device_removed"}', $client->getResponse()->getContent());
        }
    }

    public function testARemovalCanBeUndoneUntilTheDeviceIsTold(): void
    {
        $client = $this->createTestClient();
        $email = 'undo-removal@example.com';
        $laptop = $this->signInNewDevice($client, $email, deviceName: 'Laptop');
        $phone = $this->loginDevice($client, $email, 'correct horse battery', Uuid::v4()->toRfc4122(), 'Phone');

        $client->request('DELETE', '/v1/account/devices/'.$laptop['device_id'], server: $phone['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        // A stranger who knows the id, without the token, does not end the undo.
        $client->request('GET', '/v1/account', server: $this->deviceHeaders($laptop['device_id'], 'not-the-token'));
        self::assertSame('{"error":"invalid_token"}', $client->getResponse()->getContent());

        $client->request('POST', '/v1/account/devices/'.$laptop['device_id'].'/restore', server: $phone['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        $client->request('GET', '/v1/account', server: $laptop['headers']);
        self::assertResponseIsSuccessful();
    }

    public function testARemovalCannotBeUndoneOnceTheDeviceWasTold(): void
    {
        $client = $this->createTestClient();
        $email = 'undo-too-late@example.com';
        $laptop = $this->signInNewDevice($client, $email, deviceName: 'Laptop');
        $phone = $this->loginDevice($client, $email, 'correct horse battery', Uuid::v4()->toRfc4122(), 'Phone');

        $client->request('DELETE', '/v1/account/devices/'.$laptop['device_id'], server: $phone['headers']);
        $client->request('GET', '/v1/account', server: $laptop['headers']);
        self::assertSame('{"error":"device_removed"}', $client->getResponse()->getContent());

        $client->request('POST', '/v1/account/devices/'.$laptop['device_id'].'/restore', server: $phone['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_CONFLICT);
        self::assertSame('{"error":"device_already_reset"}', $client->getResponse()->getContent());
    }

    public function testADeviceThatRemovesItselfCannotBeRestored(): void
    {
        $client = $this->createTestClient();
        $email = 'self-removal@example.com';
        $laptop = $this->signInNewDevice($client, $email, deviceName: 'Laptop');
        $phone = $this->loginDevice($client, $email, 'correct horse battery', Uuid::v4()->toRfc4122(), 'Phone');

        $client->request('DELETE', '/v1/account/devices/'.$laptop['device_id'], server: $laptop['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        $client->request('POST', '/v1/account/devices/'.$laptop['device_id'].'/restore', server: $phone['headers']);
        self::assertSame('{"error":"device_already_reset"}', $client->getResponse()->getContent());
    }

    public function testRemovingSomeoneElsesDeviceFails(): void
    {
        $client = $this->createTestClient();
        $owner = $this->signInNewDevice($client, 'device-owner@example.com');
        $intruder = $this->signInNewDevice($client, 'device-intruder@example.com');

        $client->request('DELETE', '/v1/account/devices/'.$owner['device_id'], server: $intruder['headers']);

        self::assertResponseStatusCodeSame(Response::HTTP_NOT_FOUND);
        self::assertSame('{"error":"device_not_found"}', $client->getResponse()->getContent());
    }

    public function testRemovingAnUnknownDeviceFails(): void
    {
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'revoke-unknown@example.com');

        $client->request('DELETE', '/v1/account/devices/'.Uuid::v4()->toRfc4122(), server: $session['headers']);

        self::assertResponseStatusCodeSame(Response::HTTP_NOT_FOUND);
        self::assertSame('{"error":"device_not_found"}', $client->getResponse()->getContent());
    }

    public function testForgotPasswordSendsAResetCodeForAConfirmedAccountWithAPassword(): void
    {
        $client = $this->createTestClient();
        $email = $this->registerAndConfirm($client, 'forgot@example.com');

        $this->forgotPassword($client, $email);

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertSame('{}', $client->getResponse()->getContent());
        // Each request reboots the kernel, which resets the message logger
        // (docs/architecture/accounts.md), so only this request's own email
        // is still there to check.
        self::assertEmailCount(1);
        $message = self::getMailerMessage();
        self::assertInstanceOf(Email::class, $message);
        self::assertMatchesRegularExpression('/\d{6}/', (string) $message->getTextBody());
    }

    public function testForgotPasswordForAnUnconfirmedAccountSendsNothing(): void
    {
        $client = $this->createTestClient();
        $this->register($client, 'forgot-unconfirmed@example.com', 'Ada', 'en');

        $this->forgotPassword($client, 'forgot-unconfirmed@example.com');

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertEmailCount(0);
    }

    public function testForgotPasswordForAnUnknownEmailSendsNothing(): void
    {
        $client = $this->createTestClient();

        $this->forgotPassword($client, 'forgot-unknown@example.com');

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertEmailCount(0);
    }

    public function testResetPasswordWithTheRightCodeChangesItAndKeepsOnlyTheCallingDeviceSignedIn(): void
    {
        $client = $this->createTestClient();
        $email = 'reset@example.com';
        $first = $this->signInNewDevice($client, $email);
        $second = $this->loginDevice($client, $email, 'correct horse battery', Uuid::v4()->toRfc4122(), 'Phone');

        $code = $this->forgotPasswordAndGetCode($client, $email);
        $this->resetPassword($client, $email, $code, 'a brand new password', $first['headers']);

        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);
        self::assertEmailCount(1);

        // The calling device (first) stays signed in; every other device (second) is signed out.
        $client->request('GET', '/v1/account', server: $first['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_OK);
        $client->request('GET', '/v1/account', server: $second['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"signed_out_password_changed"}', $client->getResponse()->getContent());

        // Signing in again on that same device clears the reason.
        $challenge = $this->requestChallenge($client)['challenge'];
        $signature = $this->sign($challenge, $second['device_id'], sodium_base642bin($second['secret_key'], \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING));
        $this->requestLogin($client, $email, 'a brand new password', $second['device_id'], 'Phone', $second['public_key'], $challenge, $signature);
        self::assertResponseIsSuccessful();
        /** @var array{token: string} $again */
        $again = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);
        $client->request('GET', '/v1/account', server: $this->deviceHeaders($second['device_id'], $again['token']));
        self::assertResponseIsSuccessful();

        $third = $this->loginDevice($client, $email, 'a brand new password', Uuid::v4()->toRfc4122(), 'Third device');
        self::assertResponseStatusCodeSame(Response::HTTP_OK);
        self::assertNotSame('', $third['token']);
    }

    public function testResetPasswordWithNoDeviceHeadersSignsOutEveryDevice(): void
    {
        $client = $this->createTestClient();
        $email = 'reset-no-device@example.com';
        $only = $this->signInNewDevice($client, $email);

        $code = $this->forgotPasswordAndGetCode($client, $email);
        $this->resetPassword($client, $email, $code, 'a brand new password');

        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        $client->request('GET', '/v1/account', server: $only['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"signed_out_password_changed"}', $client->getResponse()->getContent());
    }

    public function testResetPasswordWithABadDeviceTokenStillSucceeds(): void
    {
        $client = $this->createTestClient();
        $email = 'reset-bad-token@example.com';
        $this->registerAndConfirmForLogin($client, $email, 'correct horse battery');

        $code = $this->forgotPasswordAndGetCode($client, $email);
        $this->resetPassword($client, $email, $code, 'a brand new password', [
            'HTTP_X-Device-Id' => Uuid::v4()->toRfc4122(),
            'HTTP_Authorization' => 'Bearer not-a-real-token',
        ]);

        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);
    }

    public function testResetPasswordWithAWrongCodeFails(): void
    {
        $client = $this->createTestClient();
        $email = 'reset-wrong-code@example.com';
        $this->registerAndConfirmForLogin($client, $email, 'correct horse battery');
        $this->forgotPasswordAndGetCode($client, $email);

        $this->resetPassword($client, $email, '000000', 'a brand new password');

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_code"}', $client->getResponse()->getContent());
    }

    public function testChangePasswordWithTheRightCurrentPasswordChangesItAndSignsOutOtherDevices(): void
    {
        $client = $this->createTestClient();
        $email = 'change@example.com';
        $first = $this->signInNewDevice($client, $email);
        $second = $this->loginDevice($client, $email, 'correct horse battery', Uuid::v4()->toRfc4122(), 'Phone');

        $this->changePassword($client, $first['headers'], 'correct horse battery', 'a brand new password');

        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);
        self::assertEmailCount(1);

        $client->request('GET', '/v1/account', server: $first['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_OK);
        $client->request('GET', '/v1/account', server: $second['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"signed_out_password_changed"}', $client->getResponse()->getContent());
    }

    public function testChangePasswordWithAWrongCurrentPasswordFails(): void
    {
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'change-wrong@example.com');

        $this->changePassword($client, $session['headers'], 'not the right password', 'a brand new password');

        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"invalid_credentials"}', $client->getResponse()->getContent());
    }

    public function testChangePasswordForAnAccountWithoutAPasswordFails(): void
    {
        $client = $this->createTestClient();
        $headers = $this->createPasswordlessDeviceSession('no-password-change@example.com');

        $this->changePassword($client, $headers, 'whatever', 'a brand new password');

        self::assertResponseStatusCodeSame(Response::HTTP_CONFLICT);
        self::assertSame('{"error":"no_password"}', $client->getResponse()->getContent());
    }

    public function testGuessingTheCurrentPasswordIsCutPerAccountLikeSigningIn(): void
    {
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'change-guess@example.com');

        // A phone left unlocked: whoever holds it tries passwords to learn the one that outlives its token.
        for ($i = 0; $i < 5; ++$i) {
            if (0 === $i % 2) {
                $this->changePassword($client, $session['headers'], 'guess number '.$i, 'a brand new password');
            } else {
                $client->request('DELETE', '/v1/account', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode(['password' => 'guess number '.$i], \JSON_THROW_ON_ERROR));
            }
            self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        }

        $this->changePassword($client, $session['headers'], 'correct horse battery', 'a brand new password');

        self::assertResponseStatusCodeSame(Response::HTTP_TOO_MANY_REQUESTS);
        self::assertNotNull($this->findUser('change-guess@example.com'));
    }

    public function testDeleteWithTheRightPasswordRemovesTheAccount(): void
    {
        $client = $this->createTestClient();
        $email = 'delete-me@example.com';
        $session = $this->signInNewDevice($client, $email);

        $client->request('DELETE', '/v1/account', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode(['password' => 'correct horse battery'], \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);
        self::assertNull($this->findUser($email));

        $client->request('GET', '/v1/account', server: $session['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
    }

    public function testDeleteWithAWrongPasswordFails(): void
    {
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'delete-wrong@example.com');

        $client->request('DELETE', '/v1/account', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode(['password' => 'not the right password'], \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"invalid_credentials"}', $client->getResponse()->getContent());

        self::assertNotNull($this->findUser('delete-wrong@example.com'));
    }

    public function testDeleteForAnAccountWithoutAPasswordIsNotDoneWithAPassword(): void
    {
        $client = $this->createTestClient();
        $headers = $this->createPasswordlessDeviceSession('no-password-delete@example.com');

        $client->request('DELETE', '/v1/account', server: [...$headers, 'CONTENT_TYPE' => 'application/json'], content: json_encode(['password' => 'whatever'], \JSON_THROW_ON_ERROR));

        // It asks for a code from its email instead (SensitiveAccountActionsTest).
        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"invalid_credentials"}', $client->getResponse()->getContent());
        self::assertNotNull($this->findUser('no-password-delete@example.com'));
    }

    private function forgotPassword(KernelBrowser $client, string $email): void
    {
        $client->request('POST', '/v1/account/password/forgot', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode(['email' => $email], \JSON_THROW_ON_ERROR));
    }

    private function forgotPasswordAndGetCode(KernelBrowser $client, string $email): string
    {
        $this->forgotPassword($client, $email);

        // Each request reboots the kernel and resets the message logger, so
        // this request's email is always the only (and first) one there.
        $message = self::getMailerMessage();
        self::assertInstanceOf(Email::class, $message);
        $body = (string) $message->getTextBody();
        preg_match('/\d{6}/', $body, $matches);

        return $matches[0] ?? self::fail('No code in the email.');
    }

    /** @param array<string, string> $headers */
    private function resetPassword(KernelBrowser $client, string $email, string $code, string $password, array $headers = []): void
    {
        $client->request('POST', '/v1/account/password/reset', server: [...$headers, 'CONTENT_TYPE' => 'application/json'], content: json_encode(['email' => $email, 'code' => $code, 'password' => $password], \JSON_THROW_ON_ERROR));
    }

    /** @param array<string, string> $headers */
    private function changePassword(KernelBrowser $client, array $headers, string $currentPassword, string $newPassword): void
    {
        $client->request('POST', '/v1/account/password', server: [...$headers, 'CONTENT_TYPE' => 'application/json'], content: json_encode(['current_password' => $currentPassword, 'new_password' => $newPassword], \JSON_THROW_ON_ERROR));
    }

    /**
     * A confirmed account with no password at all (the future Google-only
     * case), signed in with a hand-crafted device so its no_password paths
     * can be reached without going through /v1/auth/login, which always
     * needs a password to check.
     *
     * @return array{HTTP_X-Device-Id: string, HTTP_Authorization: string}
     */
    private function createPasswordlessDeviceSession(string $email): array
    {
        $entityManager = $this->entityManager();
        $user = new User($email, 'Ada', 'en');
        $user->confirm(new \DateTimeImmutable());
        $entityManager->persist($user);

        $deviceId = Uuid::v4()->toRfc4122();
        $token = bin2hex(random_bytes(16));
        $device = new Device(
            $deviceId,
            $user,
            'Test device',
            sodium_bin2base64(random_bytes(32), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING),
            hash('sha256', $token),
        );
        $entityManager->persist($device);
        $entityManager->flush();

        return [
            'HTTP_X-Device-Id' => $deviceId,
            'HTTP_Authorization' => 'Bearer '.$token,
        ];
    }

    private function findUser(string $email): ?User
    {
        return $this->entityManager()->getRepository(User::class)->findOneBy(['email' => $email]);
    }

    private function entityManager(): EntityManagerInterface
    {
        return self::getContainer()->get(EntityManagerInterface::class);
    }
}
