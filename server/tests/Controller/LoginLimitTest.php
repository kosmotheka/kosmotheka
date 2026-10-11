<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Tests\Support\DeviceAuthTestTrait;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\Uid\Uuid;

/**
 * SEG-051: the limit of sign-ins is not the one of a house. Failures are counted per account (5 every 15 minutes, also
 * for an address with no account), and the limit per address leaves room for a family behind one router.
 */
final class LoginLimitTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string PASSWORD = 'correct horse battery';

    private function createTestClient(): KernelBrowser
    {
        $client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();

        return $client;
    }

    /**
     * One attempt from the given address; returns the status and the body.
     *
     * @return array{int, string}
     */
    private function signInAttempt(KernelBrowser $client, string $address, string $email, string $password): array
    {
        $client->setServerParameter('REMOTE_ADDR', $address);
        $challenge = $this->requestChallenge($client)['challenge'];
        $keyPair = sodium_crypto_sign_keypair();
        $deviceId = Uuid::v4()->toRfc4122();
        $this->requestLogin(
            $client,
            $email,
            $password,
            $deviceId,
            'Phone',
            $this->encode(sodium_crypto_sign_publickey($keyPair)),
            $challenge,
            $this->sign($challenge, $deviceId, sodium_crypto_sign_secretkey($keyPair)),
        );

        return [$client->getResponse()->getStatusCode(), (string) $client->getResponse()->getContent()];
    }

    private function account(KernelBrowser $client, string $email): void
    {
        $client->setServerParameter('REMOTE_ADDR', '198.51.100.1');
        $this->registerAndConfirmForLogin($client, $email, self::PASSWORD);
    }

    public function testSixFailuresAgainstOneAccountFromDifferentAddressesAreCutEvenWithTheRightPassword(): void
    {
        $client = $this->createTestClient();
        $this->account($client, 'target@example.com');

        for ($i = 1; $i <= 5; ++$i) {
            [$status] = $this->signInAttempt($client, '203.0.113.'.$i, 'target@example.com', 'not the password');
            self::assertSame(Response::HTTP_UNAUTHORIZED, $status);
        }

        [$status, $body] = $this->signInAttempt($client, '203.0.113.99', 'target@example.com', self::PASSWORD);
        self::assertSame(Response::HTTP_TOO_MANY_REQUESTS, $status);
        self::assertSame('{"error":"too_many_requests"}', $body);
    }

    public function testAnAddressWithNoAccountBehavesTheSameAsOneWithAnAccount(): void
    {
        $client = $this->createTestClient();
        $this->account($client, 'real@example.com');

        $answers = ['real@example.com' => [], 'ghost@example.com' => []];
        foreach (array_keys($answers) as $email) {
            for ($i = 1; $i <= 7; ++$i) {
                $answers[$email][] = $this->signInAttempt($client, '203.0.113.'.$i, $email, 'not the password');
            }
        }

        self::assertSame($answers['real@example.com'], $answers['ghost@example.com']);
        self::assertSame([401, 401, 401, 401, 401, 429, 429], array_column($answers['ghost@example.com'], 0));
    }

    public function testTheAddressIsCountedWithoutCapitals(): void
    {
        $client = $this->createTestClient();
        $this->account($client, 'case@example.com');

        foreach (['case@example.com', 'CASE@example.com', 'Case@Example.com', 'cAsE@example.com', 'case@EXAMPLE.com'] as $i => $email) {
            $this->signInAttempt($client, '203.0.113.'.($i + 1), $email, 'not the password');
        }

        [$status] = $this->signInAttempt($client, '203.0.113.99', 'case@example.com', self::PASSWORD);
        self::assertSame(Response::HTTP_TOO_MANY_REQUESTS, $status);
    }

    public function testSeveralDevicesOfOneHouseSignInOneAfterAnotherWithoutACut(): void
    {
        $client = $this->createTestClient();
        $this->account($client, 'house@example.com');

        for ($i = 0; $i < 6; ++$i) {
            [$status] = $this->signInAttempt($client, '203.0.113.7', 'house@example.com', self::PASSWORD);
            self::assertSame(Response::HTTP_OK, $status, 'sign-in number '.($i + 1));
        }
    }

    public function testASuccessfulSignInDoesNotSpendTheFailuresOfTheAccount(): void
    {
        $client = $this->createTestClient();
        $this->account($client, 'spend@example.com');

        for ($i = 1; $i <= 4; ++$i) {
            $this->signInAttempt($client, '203.0.113.'.$i, 'spend@example.com', 'not the password');
        }
        for ($i = 0; $i < 4; ++$i) {
            [$status] = $this->signInAttempt($client, '203.0.113.50', 'spend@example.com', self::PASSWORD);
            self::assertSame(Response::HTTP_OK, $status);
        }

        // The fifth failure is still an answer, the sixth attempt is the cut.
        [$fifth] = $this->signInAttempt($client, '203.0.113.60', 'spend@example.com', 'not the password');
        [$sixth] = $this->signInAttempt($client, '203.0.113.61', 'spend@example.com', self::PASSWORD);
        self::assertSame(Response::HTTP_UNAUTHORIZED, $fifth);
        self::assertSame(Response::HTTP_TOO_MANY_REQUESTS, $sixth);
    }

    public function testTheCutOfOneAccountDoesNotReachAnotherOneOfTheSameAddress(): void
    {
        $client = $this->createTestClient();
        $this->account($client, 'victim@example.com');
        $this->account($client, 'neighbour@example.com');

        for ($i = 0; $i < 6; ++$i) {
            $this->signInAttempt($client, '203.0.113.7', 'victim@example.com', 'not the password');
        }

        [$status] = $this->signInAttempt($client, '203.0.113.7', 'neighbour@example.com', self::PASSWORD);
        self::assertSame(Response::HTTP_OK, $status);
    }
}
