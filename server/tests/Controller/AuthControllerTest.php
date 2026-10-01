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
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\Uid\Uuid;

/**
 * Functional coverage of `/v1/auth/*` and the device-token firewall
 * (milestone 7.1, part 2, docs/architecture/accounts.md).
 */
final class AuthControllerTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

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

        $device = $this->deviceRepository()->find($session['device_id']);
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

        $device = $this->deviceRepository()->find($deviceId);
        self::assertInstanceOf(Device::class, $device);
        self::assertSame('Second name', $device->getName());

        // The old token no longer works.
        $client->request('GET', '/v1/account', server: $firstHeaders);
        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"invalid_token"}', $client->getResponse()->getContent());
    }

    public function testADeviceTakenByAnotherAccountIsRejected(): void
    {
        $client = $this->createTestClient();
        $firstEmail = 'owner@example.com';
        $this->registerAndConfirmForLogin($client, $firstEmail, 'correct horse battery');
        $deviceId = Uuid::v4()->toRfc4122();
        $this->loginDevice($client, $firstEmail, 'correct horse battery', $deviceId, 'Shared device');

        $secondEmail = 'intruder@example.com';
        $this->registerAndConfirmForLogin($client, $secondEmail, 'correct horse battery');
        $challenge = $this->requestChallenge($client)['challenge'];
        $keyPair = sodium_crypto_sign_keypair();
        $signature = $this->sign($challenge, $deviceId, sodium_crypto_sign_secretkey($keyPair));
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));

        $this->requestLogin($client, $secondEmail, 'correct horse battery', $deviceId, 'Shared device', $publicKey, $challenge, $signature);

        self::assertResponseStatusCodeSame(Response::HTTP_CONFLICT);
        self::assertSame('{"error":"device_taken"}', $client->getResponse()->getContent());
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
