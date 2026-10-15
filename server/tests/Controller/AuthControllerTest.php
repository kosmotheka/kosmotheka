<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Entity\Device;
use App\Entity\User;
use App\Repository\DeviceRepository;
use App\Tests\Support\DeviceAuthTestTrait;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\Clock\Test\ClockSensitiveTrait;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\Uid\Uuid;

/**
 * Functional coverage of `/v1/auth/*` and the device-token firewall
 * (milestone 7.1, part 2, docs/architecture/accounts.md).
 */
final class AuthControllerTest extends WebTestCase
{
    use ClockSensitiveTrait;
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string AT = '2026-01-02 12:00:00';
    // One TOTP period later: enabling 2FA already consumes AT's timestep
    // (anti-replay, D78), so logging in needs a code from a later one.
    private const string LOGIN_AT = '2026-01-02 12:00:30';

    /** See AccountControllerTest::createTestClient() for why this happens here, not in setUp(). */
    private function createTestClient(): KernelBrowser
    {
        $client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();

        return $client;
    }

    public function testChallengeIsSingleUse(): void
    {
        $client = $this->createTestClient();
        $challenge = $this->requestChallenge($client)['challenge'];
        $email = 'challenge-reuse@example.com';
        $this->registerAndConfirmForLogin($client, $email, 'correct horse battery');
        $keyPair = sodium_crypto_sign_keypair();
        $deviceId = Uuid::v4()->toRfc4122();
        $signature = $this->sign($challenge, $deviceId, sodium_crypto_sign_secretkey($keyPair));
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));

        $this->requestLogin($client, $email, 'correct horse battery', $deviceId, 'Phone', $publicKey, $challenge, $signature);
        self::assertResponseStatusCodeSame(Response::HTTP_OK);

        $this->requestLogin($client, $email, 'correct horse battery', $deviceId, 'Phone', $publicKey, $challenge, $signature);
        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_challenge"}', $client->getResponse()->getContent());
    }

    public function testLoginWithAnUnknownChallengeFails(): void
    {
        $client = $this->createTestClient();
        $email = 'unknown-challenge@example.com';
        $this->registerAndConfirmForLogin($client, $email, 'correct horse battery');
        $keyPair = sodium_crypto_sign_keypair();
        $deviceId = Uuid::v4()->toRfc4122();
        $challenge = $this->encode(random_bytes(32));
        $signature = $this->sign($challenge, $deviceId, sodium_crypto_sign_secretkey($keyPair));
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));

        $this->requestLogin($client, $email, 'correct horse battery', $deviceId, 'Phone', $publicKey, $challenge, $signature);

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_challenge"}', $client->getResponse()->getContent());
    }

    public function testLoginWithAnExpiredChallengeFails(): void
    {
        $client = $this->createTestClient();
        $challenge = $this->requestChallenge($client)['challenge'];
        self::getContainer()->get('cache.auth_challenge')->delete('login_challenge_'.$challenge);
        $email = 'expired-challenge@example.com';
        $this->registerAndConfirmForLogin($client, $email, 'correct horse battery');
        $keyPair = sodium_crypto_sign_keypair();
        $deviceId = Uuid::v4()->toRfc4122();
        $signature = $this->sign($challenge, $deviceId, sodium_crypto_sign_secretkey($keyPair));
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));

        $this->requestLogin($client, $email, 'correct horse battery', $deviceId, 'Phone', $publicKey, $challenge, $signature);

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_challenge"}', $client->getResponse()->getContent());
    }

    public function testLoginWithAWrongSignatureFails(): void
    {
        $client = $this->createTestClient();
        $email = 'wrong-signature@example.com';
        $this->registerAndConfirmForLogin($client, $email, 'correct horse battery');
        $challenge = $this->requestChallenge($client)['challenge'];
        $keyPair = sodium_crypto_sign_keypair();
        $otherKeyPair = sodium_crypto_sign_keypair();
        $deviceId = Uuid::v4()->toRfc4122();
        // Signed with a key that does not match the public key sent along.
        $signature = $this->sign($challenge, $deviceId, sodium_crypto_sign_secretkey($otherKeyPair));
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));

        $this->requestLogin($client, $email, 'correct horse battery', $deviceId, 'Phone', $publicKey, $challenge, $signature);

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_device_proof"}', $client->getResponse()->getContent());
    }

    public function testLoginWithASignatureForAnotherDeviceIdFails(): void
    {
        $client = $this->createTestClient();
        $email = 'wrong-device-id@example.com';
        $this->registerAndConfirmForLogin($client, $email, 'correct horse battery');
        $challenge = $this->requestChallenge($client)['challenge'];
        $keyPair = sodium_crypto_sign_keypair();
        $signature = $this->sign($challenge, Uuid::v4()->toRfc4122(), sodium_crypto_sign_secretkey($keyPair));
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));

        $this->requestLogin($client, $email, 'correct horse battery', Uuid::v4()->toRfc4122(), 'Phone', $publicKey, $challenge, $signature);

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_device_proof"}', $client->getResponse()->getContent());
    }

    public function testLoginWithAnUnknownEmailFails(): void
    {
        $client = $this->createTestClient();
        $challenge = $this->requestChallenge($client)['challenge'];
        $keyPair = sodium_crypto_sign_keypair();
        $deviceId = Uuid::v4()->toRfc4122();
        $signature = $this->sign($challenge, $deviceId, sodium_crypto_sign_secretkey($keyPair));
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));

        $this->requestLogin($client, 'nobody@example.com', 'whatever password', $deviceId, 'Phone', $publicKey, $challenge, $signature);

        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"invalid_credentials"}', $client->getResponse()->getContent());
    }

    public function testLoginWithAWrongPasswordFails(): void
    {
        $client = $this->createTestClient();
        $email = 'wrong-password@example.com';
        $this->registerAndConfirmForLogin($client, $email, 'correct horse battery');
        $challenge = $this->requestChallenge($client)['challenge'];
        $keyPair = sodium_crypto_sign_keypair();
        $deviceId = Uuid::v4()->toRfc4122();
        $signature = $this->sign($challenge, $deviceId, sodium_crypto_sign_secretkey($keyPair));
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));

        $this->requestLogin($client, $email, 'not the right password', $deviceId, 'Phone', $publicKey, $challenge, $signature);

        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"invalid_credentials"}', $client->getResponse()->getContent());
    }

    public function testLoginOfAnUnconfirmedAccountFails(): void
    {
        $client = $this->createTestClient();
        $email = 'unconfirmed-login@example.com';
        $client->request('POST', '/v1/account/register', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode([
            'email' => $email,
            'password' => 'correct horse battery',
            'name' => 'Ada',
            'locale' => 'en',
        ], \JSON_THROW_ON_ERROR));
        $challenge = $this->requestChallenge($client)['challenge'];
        $keyPair = sodium_crypto_sign_keypair();
        $deviceId = Uuid::v4()->toRfc4122();
        $signature = $this->sign($challenge, $deviceId, sodium_crypto_sign_secretkey($keyPair));
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));

        $this->requestLogin($client, $email, 'correct horse battery', $deviceId, 'Phone', $publicKey, $challenge, $signature);

        self::assertResponseStatusCodeSame(Response::HTTP_FORBIDDEN);
        self::assertSame('{"error":"account_not_confirmed"}', $client->getResponse()->getContent());
    }

    public function testLoginOfABlockedAccountFails(): void
    {
        $client = $this->createTestClient();
        $email = 'blocked-login@example.com';
        $this->registerAndConfirmForLogin($client, $email, 'correct horse battery');
        $user = $this->findUser($email);
        self::assertNotNull($user);
        new \ReflectionProperty($user, 'blockedAt')->setValue($user, new \DateTimeImmutable());
        $this->entityManager()->flush();

        $challenge = $this->requestChallenge($client)['challenge'];
        $keyPair = sodium_crypto_sign_keypair();
        $deviceId = Uuid::v4()->toRfc4122();
        $signature = $this->sign($challenge, $deviceId, sodium_crypto_sign_secretkey($keyPair));
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));

        $this->requestLogin($client, $email, 'correct horse battery', $deviceId, 'Phone', $publicKey, $challenge, $signature);

        self::assertResponseStatusCodeSame(Response::HTTP_FORBIDDEN);
        self::assertSame('{"error":"account_blocked"}', $client->getResponse()->getContent());
    }

    public function testLoginSucceedsAndCreatesTheDevice(): void
    {
        $client = $this->createTestClient();

        $session = $this->signInNewDevice($client, 'new-device@example.com');

        self::assertResponseStatusCodeSame(Response::HTTP_OK);
        /** @var array{token: string, user: array<string, mixed>} $body */
        $body = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);
        self::assertSame('new-device@example.com', $body['user']['email']);
        self::assertSame($session['token'], $body['token']);

        $device = $this->deviceRepository()->findOneBy(['deviceId' => $session['device_id']]);
        self::assertInstanceOf(Device::class, $device);
        self::assertSame('Test device', $device->getName());
        self::assertTrue($device->isSignedIn());
    }

    public function testLoggingInAgainWithTheSameDeviceRotatesItsTokenAndUpdatesItsName(): void
    {
        $client = $this->createTestClient();
        $email = 'rotate@example.com';
        $this->registerAndConfirmForLogin($client, $email, 'correct horse battery');
        $deviceId = Uuid::v4()->toRfc4122();
        $keyPair = sodium_crypto_sign_keypair();
        $secretKey = sodium_crypto_sign_secretkey($keyPair);
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));

        $firstChallenge = $this->requestChallenge($client)['challenge'];
        $this->requestLogin($client, $email, 'correct horse battery', $deviceId, 'First name', $publicKey, $firstChallenge, $this->sign($firstChallenge, $deviceId, $secretKey));
        /** @var array{token: string} $firstBody */
        $firstBody = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);
        $firstHeaders = $this->deviceHeaders($deviceId, $firstBody['token']);

        $secondChallenge = $this->requestChallenge($client)['challenge'];
        $this->requestLogin($client, $email, 'correct horse battery', $deviceId, 'Second name', $publicKey, $secondChallenge, $this->sign($secondChallenge, $deviceId, $secretKey));
        self::assertResponseStatusCodeSame(Response::HTTP_OK);
        /** @var array{token: string} $secondBody */
        $secondBody = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);
        self::assertNotSame($firstBody['token'], $secondBody['token']);

        $device = $this->deviceRepository()->findOneBy(['deviceId' => $deviceId]);
        self::assertInstanceOf(Device::class, $device);
        self::assertSame('Second name', $device->getName());

        // The old token no longer works.
        $client->request('GET', '/v1/account', server: $firstHeaders);
        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"invalid_token"}', $client->getResponse()->getContent());
    }

    /**
     * A device is its id together with its key (D84, seam 6). A second account
     * logging in with the same id and another key is not a takeover any more:
     * it is another install, gets its own row, and the first device keeps working.
     */
    public function testAnotherAccountWithTheSameIdAndAnotherKeyGetsItsOwnDevice(): void
    {
        $client = $this->createTestClient();
        $deviceId = Uuid::v4()->toRfc4122();
        $this->registerAndConfirmForLogin($client, 'owner@example.com', 'correct horse battery');
        $first = $this->loginDevice($client, 'owner@example.com', 'correct horse battery', $deviceId, 'First');

        $this->registerAndConfirmForLogin($client, 'other@example.com', 'correct horse battery');
        $second = $this->loginDevice($client, 'other@example.com', 'correct horse battery', $deviceId, 'Second');
        self::assertResponseStatusCodeSame(Response::HTTP_OK);

        self::assertCount(2, $this->deviceRepository()->findBy(['deviceId' => $deviceId]));

        $client->request('GET', '/v1/account', server: $first['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_OK);
        $client->request('GET', '/v1/account', server: $second['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_OK);
    }

    public function testTheSameIdAndKeyInASecondAccountIsDeviceTaken(): void
    {
        $client = $this->createTestClient();
        $email = 'owner@example.com';
        $this->registerAndConfirmForLogin($client, $email, 'correct horse battery');
        $deviceId = Uuid::v4()->toRfc4122();
        $first = $this->loginDevice($client, $email, 'correct horse battery', $deviceId, 'Shared device');

        $secondEmail = 'second@example.com';
        $this->registerAndConfirmForLogin($client, $secondEmail, 'correct horse battery');
        $challenge = $this->requestChallenge($client)['challenge'];
        $secretKey = sodium_base642bin($first['secret_key'], \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        $signature = $this->sign($challenge, $deviceId, $secretKey);

        $this->requestLogin($client, $secondEmail, 'correct horse battery', $deviceId, 'Shared device', $first['public_key'], $challenge, $signature);

        self::assertResponseStatusCodeSame(Response::HTTP_CONFLICT);
        self::assertSame('{"error":"device_taken"}', $client->getResponse()->getContent());
    }

    /**
     * D84, seam 6: someone who learnt the id of a phone signs in with it first,
     * with a key of their own. The phone, proving its real key, must still get in.
     */
    public function testAnIntruderCannotTakeADeviceIdBeforeItsOwner(): void
    {
        $client = $this->createTestClient();
        $deviceId = Uuid::v4()->toRfc4122();
        $this->registerAndConfirmForLogin($client, 'intruder@example.com', 'correct horse battery');
        $this->loginDevice($client, 'intruder@example.com', 'correct horse battery', $deviceId, 'Not mine');
        self::assertResponseIsSuccessful();

        $this->registerAndConfirmForLogin($client, 'owner@example.com', 'correct horse battery');
        $this->loginDevice($client, 'owner@example.com', 'correct horse battery', $deviceId, 'My phone');

        self::assertResponseIsSuccessful();
    }

    public function testADeviceIdWithADifferentKeyIsRejected(): void
    {
        $client = $this->createTestClient();
        $email = 'key-mismatch@example.com';
        $this->registerAndConfirmForLogin($client, $email, 'correct horse battery');
        $deviceId = Uuid::v4()->toRfc4122();
        $this->loginDevice($client, $email, 'correct horse battery', $deviceId, 'Phone');

        $challenge = $this->requestChallenge($client)['challenge'];
        $otherKeyPair = sodium_crypto_sign_keypair();
        $signature = $this->sign($challenge, $deviceId, sodium_crypto_sign_secretkey($otherKeyPair));
        $publicKey = $this->encode(sodium_crypto_sign_publickey($otherKeyPair));

        $this->requestLogin($client, $email, 'correct horse battery', $deviceId, 'Phone', $publicKey, $challenge, $signature);

        self::assertResponseStatusCodeSame(Response::HTTP_CONFLICT);
        self::assertSame('{"error":"device_key_mismatch"}', $client->getResponse()->getContent());
    }

    /** SEG-029: one spelling per device, the one operations carry. */
    public function testADeviceIdInCapitalsCannotSignIn(): void
    {
        $client = $this->createTestClient();
        $email = 'capitals@example.com';
        $this->registerAndConfirmForLogin($client, $email, 'correct horse battery');
        $deviceId = strtoupper(Uuid::v4()->toRfc4122());
        $keyPair = sodium_crypto_sign_keypair();
        $challenge = $this->requestChallenge($client)['challenge'];

        $this->requestLogin($client, $email, 'correct horse battery', $deviceId, 'Phone', $this->encode(sodium_crypto_sign_publickey($keyPair)), $challenge, $this->sign($challenge, $deviceId, sodium_crypto_sign_secretkey($keyPair)));

        self::assertResponseStatusCodeSame(Response::HTTP_UNPROCESSABLE_ENTITY);
        self::assertNull($this->deviceRepository()->findOneBy(['deviceId' => $deviceId]));
    }

    public function testLogoutInvalidatesTheToken(): void
    {
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'logout@example.com');

        $client->request('POST', '/v1/auth/logout', server: $session['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        $client->request('GET', '/v1/account', server: $session['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"invalid_token"}', $client->getResponse()->getContent());
    }

    public function testProtectedRouteWithoutCredentialsFails(): void
    {
        $client = $this->createTestClient();

        $client->request('GET', '/v1/account');

        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"authentication_required"}', $client->getResponse()->getContent());
    }

    public function testProtectedRouteWithAnUnknownDeviceFails(): void
    {
        $client = $this->createTestClient();

        $client->request('GET', '/v1/account', server: [
            'HTTP_X-Device-Id' => Uuid::v4()->toRfc4122(),
            'HTTP_Authorization' => 'Bearer '.$this->encode(random_bytes(32)),
        ]);

        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"invalid_token"}', $client->getResponse()->getContent());
    }

    public function testProtectedRouteWithAWrongTokenFails(): void
    {
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'wrong-token@example.com');

        $client->request('GET', '/v1/account', server: [
            'HTTP_X-Device-Id' => $session['device_id'],
            'HTTP_Authorization' => 'Bearer '.$this->encode(random_bytes(32)),
        ]);

        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"invalid_token"}', $client->getResponse()->getContent());
    }

    public function testProtectedRouteSucceedsWithValidCredentials(): void
    {
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'protected@example.com');

        $client->request('GET', '/v1/account', server: $session['headers']);

        self::assertResponseStatusCodeSame(Response::HTTP_OK);
    }

    public function testLoginWithTwoFactorEnabledAndNoCodeFails(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $email = 'totp-required@example.com';
        $session = $this->signInNewDevice($client, $email);
        $this->enableTwoFactor($client, $session['headers']);

        $this->loginAgainWithTotp($client, $session, null);

        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"two_factor_required"}', $client->getResponse()->getContent());
    }

    public function testLoginWithAWrongTotpCodeFails(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $email = 'totp-wrong@example.com';
        $session = $this->signInNewDevice($client, $email);
        $this->enableTwoFactor($client, $session['headers']);

        $this->loginAgainWithTotp($client, $session, '000000');

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_two_factor_code"}', $client->getResponse()->getContent());
    }

    public function testWrongSecondFactorCodesCountAsFailuresOfTheAccount(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'totp-count@example.com');
        $secret = $this->enableTwoFactor($client, $session['headers']);

        // Three wrong codes (the second factor's own limit is five) and two wrong passwords: five failures of the account.
        for ($i = 0; $i < 3; ++$i) {
            $this->loginAgainWithTotp($client, $session, '000000');
            self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        }
        for ($i = 0; $i < 2; ++$i) {
            $challenge = $this->requestChallenge($client)['challenge'];
            $this->requestLogin($client, $session['email'], 'not the password', $session['device_id'], 'Test device', $session['public_key'], $challenge, $this->sign($challenge, $session['device_id'], $this->decode($session['secret_key'])));
            self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        }

        self::mockTime(self::LOGIN_AT);
        $this->loginAgainWithTotp($client, $session, self::totpAt($secret, self::LOGIN_AT));
        self::assertResponseStatusCodeSame(Response::HTTP_TOO_MANY_REQUESTS);
    }

    public function testLoginWithTheRightTotpCodeSucceeds(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $email = 'totp-right@example.com';
        $session = $this->signInNewDevice($client, $email);
        $secret = $this->enableTwoFactor($client, $session['headers']);

        self::mockTime(self::LOGIN_AT);
        $this->loginAgainWithTotp($client, $session, self::totpAt($secret, self::LOGIN_AT));

        self::assertResponseStatusCodeSame(Response::HTTP_OK);
    }

    public function testReplayingTheSameTotpCodeAtLoginFails(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $email = 'totp-replay@example.com';
        $session = $this->signInNewDevice($client, $email);
        $secret = $this->enableTwoFactor($client, $session['headers']);

        self::mockTime(self::LOGIN_AT);
        $code = self::totpAt($secret, self::LOGIN_AT);

        $this->loginAgainWithTotp($client, $session, $code);
        self::assertResponseStatusCodeSame(Response::HTTP_OK);

        $this->loginAgainWithTotp($client, $session, $code);
        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_two_factor_code"}', $client->getResponse()->getContent());
    }

    public function testLoggingInWithABackupCodeWorksOnceAndNotTwice(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $email = 'totp-backup-login@example.com';
        $session = $this->signInNewDevice($client, $email);
        $backupCodes = $this->enableTwoFactorForBackupCodes($client, $session['headers']);

        $this->loginAgainWithTotp($client, $session, $backupCodes[0]);
        self::assertResponseStatusCodeSame(Response::HTTP_OK);

        $this->loginAgainWithTotp($client, $session, $backupCodes[0]);
        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_two_factor_code"}', $client->getResponse()->getContent());
    }

    /**
     * @param array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} $session
     */
    private function loginAgainWithTotp(KernelBrowser $client, array $session, ?string $totpCode): void
    {
        $challenge = $this->requestChallenge($client)['challenge'];
        $signature = $this->sign($challenge, $session['device_id'], $this->decode($session['secret_key']));

        $payload = [
            'email' => $session['email'],
            'password' => 'correct horse battery',
            'device_id' => $session['device_id'],
            'device_name' => 'Test device',
            'public_key' => $session['public_key'],
            'challenge' => $challenge,
            'signature' => $signature,
        ];
        if (null !== $totpCode) {
            $payload['totp_code'] = $totpCode;
        }

        $client->request('POST', '/v1/auth/login', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode($payload, \JSON_THROW_ON_ERROR));
    }

    private function decode(string $base64url): string
    {
        return sodium_base642bin($base64url, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
    }

    /** @param array<string, string> $headers */
    private function setupTwoFactor(KernelBrowser $client, array $headers): string
    {
        $client->request('POST', '/v1/account/two-factor/setup', server: $headers);

        /** @var array{secret: string} $body */
        $body = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);

        return $body['secret'];
    }

    /**
     * @param array<string, string> $headers
     *
     * @return list<string> the backup codes
     */
    private function enableTwoFactorForBackupCodes(KernelBrowser $client, array $headers): array
    {
        $secret = $this->setupTwoFactor($client, $headers);
        $code = self::totpAt($secret, self::AT);

        $client->request('POST', '/v1/account/two-factor/enable', server: [...$headers, 'CONTENT_TYPE' => 'application/json'], content: json_encode(['code' => $code, 'password' => 'correct horse battery'], \JSON_THROW_ON_ERROR));

        /** @var array{backup_codes: list<string>} $body */
        $body = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);

        return $body['backup_codes'];
    }

    /** @param array<string, string> $headers */
    private function enableTwoFactor(KernelBrowser $client, array $headers): string
    {
        $secret = $this->setupTwoFactor($client, $headers);
        $code = self::totpAt($secret, self::AT);

        $client->request('POST', '/v1/account/two-factor/enable', server: [...$headers, 'CONTENT_TYPE' => 'application/json'], content: json_encode(['code' => $code, 'password' => 'correct horse battery'], \JSON_THROW_ON_ERROR));

        return $secret;
    }

    private function findUser(string $email): ?User
    {
        return $this->entityManager()->getRepository(User::class)->findOneBy(['email' => $email]);
    }

    private function deviceRepository(): DeviceRepository
    {
        return self::getContainer()->get(DeviceRepository::class);
    }

    private function entityManager(): EntityManagerInterface
    {
        return self::getContainer()->get(EntityManagerInterface::class);
    }
}
