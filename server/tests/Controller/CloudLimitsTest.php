<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Entity\User;
use App\Tests\Support\DeviceAuthTestTrait;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\RateLimiter\RateLimiterFactoryInterface;
use Symfony\Component\Uid\Uuid;

/**
 * The cloud's routes have limits per account (decided on 2026-10-15): uploading and downloading 600 an hour each,
 * claiming and linking 30 an hour together, the token of the live notices 120 an hour. They are spent before anything
 * is looked up, and one account's limit is not another's.
 */
final class CloudLimitsTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private function createTestClient(): KernelBrowser
    {
        $client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();

        return $client;
    }

    /** @return iterable<string, array{string, string, string, int}> */
    public static function limitedRoutes(): iterable
    {
        $library = Uuid::v4()->toRfc4122();

        yield 'upload' => ['cloud_sync', 'push', "/v1/cloud/libraries/$library/push", 600];
        yield 'download' => ['cloud_sync', 'pull', "/v1/cloud/libraries/$library/pull", 600];
        yield 'link' => ['cloud_link', '', "/v1/cloud/libraries/$library/link", 30];
        yield 'claim' => ['cloud_link', '', '/v1/cloud/libraries/claim', 30];
    }

    #[DataProvider('limitedRoutes')]
    public function testAnAccountPastTheLimitOfARouteIsCutAndTheOthersAreNot(string $limiter, string $kind, string $path, int $limit): void
    {
        $client = $this->createTestClient();
        $ada = $this->signInNewDevice($client, 'ada@example.com');
        $grace = $this->signInNewDevice($client, 'grace@example.com');
        $key = $this->userId('ada@example.com').('' === $kind ? '' : '|'.$kind);
        $this->limiter($limiter)->create($key)->consume($limit);

        $this->post($client, $ada['headers'], $path);
        self::assertResponseStatusCodeSame(Response::HTTP_TOO_MANY_REQUESTS);
        self::assertSame('{"error":"too_many_requests"}', $client->getResponse()->getContent());

        $this->post($client, $grace['headers'], $path);
        self::assertNotSame(Response::HTTP_TOO_MANY_REQUESTS, $client->getResponse()->getStatusCode());
    }

    public function testTheTokenOfTheNoticesIsLimitedPerAccount(): void
    {
        $client = $this->createTestClient();
        $ada = $this->signInNewDevice($client, 'ada@example.com');
        $grace = $this->signInNewDevice($client, 'grace@example.com');
        $this->limiter('cloud_events')->create($this->userId('ada@example.com'))->consume(120);

        $client->request('GET', '/v1/cloud/events', server: $ada['headers']);
        self::assertResponseStatusCodeSame(Response::HTTP_TOO_MANY_REQUESTS);

        $client->request('GET', '/v1/cloud/events', server: $grace['headers']);
        self::assertNotSame(Response::HTTP_TOO_MANY_REQUESTS, $client->getResponse()->getStatusCode());
    }

    /** @param array<string, string> $headers */
    private function post(KernelBrowser $client, array $headers, string $path): void
    {
        $client->request('POST', $path, server: $headers + ['CONTENT_TYPE' => 'application/json'], content: json_encode(['schemaVersion' => 1, 'vector' => []], \JSON_THROW_ON_ERROR));
    }

    private function limiter(string $name): RateLimiterFactoryInterface
    {
        $factory = self::getContainer()->get('limiter.'.$name);
        self::assertInstanceOf(RateLimiterFactoryInterface::class, $factory);

        return $factory;
    }

    private function userId(string $email): string
    {
        $user = self::getContainer()->get(EntityManagerInterface::class)->getRepository(User::class)->findOneBy(['email' => $email]) ?? self::fail('No such user.');

        return $user->getId()->toRfc4122();
    }
}
