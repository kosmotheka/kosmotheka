<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\DataFixtures\SigningDevice;
use App\Tests\Support\DeviceAuthTestTrait;
use App\Tests\Support\RecordingHub;
use App\Tests\Support\Spec;
use Doctrine\DBAL\Connection;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\Uid\Uuid;

/**
 * Live notices (milestone 7.3, D75, docs/architecture/cloud.md). The cast is the one of the
 * «Nube» battery: a family, and Jhonny, who has an account of his own and knows the family's ids.
 * The hub is RecordingHub, which keeps what would be published.
 */
final class CloudEventsControllerTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string FOUNDER = '01997c7e-0000-7000-8000-000000000011';
    private const int T = 1_727_000_000_000;
    private const string TOPIC_PREFIX = 'urn:kosmotheka:';

    private KernelBrowser $client;

    /** @var array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private array $familySession;

    /** @var array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private array $jhonnySession;

    private SigningDevice $laptop;
    private SigningDevice $jhonny;

    protected function setUp(): void
    {
        RecordingHub::reset();
        $this->start();
    }

    protected function tearDown(): void
    {
        RecordingHub::reset();
        $this->setHub('http://mercure.test/.well-known/mercure');
        parent::tearDown();
    }

    // ------------------------------------------------------- the token ---

    /** Battery: Jhonny knows the family's ids and gets none of its topics. */
    public function testJhonnysTokenHoldsOnlyHisOwnTopicsWhateverIdsHeSends(): void
    {
        $familyCloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $jhonnyCloud = $this->createCloudLibrary($this->jhonnySession);
        $jhonnyId = $this->userId('jhonny@example.com');

        $attempts = [
            $this->call('GET', '/v1/cloud/events', $this->jhonnySession),
            $this->call('GET', "/v1/cloud/events?library=$familyCloud&topic=urn:kosmotheka:cloud-library:$familyCloud&id=$familyCloud", $this->jhonnySession),
            $this->call('GET', '/v1/cloud/events', [
                'headers' => $this->jhonnySession['headers'] + ['HTTP_X-Library-Id' => $familyCloud, 'HTTP_X-Cloud-Library-Id' => $familyCloud, 'HTTP_X-User-Id' => $this->userId('family@example.com')],
            ], ['library' => $familyCloud, 'topics' => [self::TOPIC_PREFIX.'cloud-library:'.$familyCloud]]),
        ];

        foreach ($attempts as $response) {
            self::assertSame(Response::HTTP_OK, $response['status']);
            $expected = [self::TOPIC_PREFIX.'account:'.$jhonnyId, self::TOPIC_PREFIX.'cloud-library:'.$jhonnyCloud];
            self::assertSame($expected, $response['body']['topics']);
            self::assertSame($expected, $this->subscribedTopics($this->tokenOf($response['body'])));
            self::assertStringNotContainsString($familyCloud, json_encode($response['body'], \JSON_THROW_ON_ERROR));
        }
    }

    public function testTheTokenCoversTheAccountAndItsLibrariesAndExpiresInAnHour(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $other = $this->createCloudLibrary($this->familySession);

        $before = time();
        $response = $this->call('GET', '/v1/cloud/events', $this->familySession);

        self::assertSame(Response::HTTP_OK, $response['status']);
        $topics = [self::TOPIC_PREFIX.'account:'.$this->userId('family@example.com'), self::TOPIC_PREFIX.'cloud-library:'.$cloud, self::TOPIC_PREFIX.'cloud-library:'.$other];
        self::assertSame($topics, $response['body']['topics']);
        $claims = $this->claimsOf($this->tokenOf($response['body']));
        self::assertSame($topics, $this->subscribedTopics($this->tokenOf($response['body'])));
        $expires = $claims['exp'];
        \assert(\is_int($expires));
        self::assertEqualsWithDelta($before + 3600, $expires, 5);
        self::assertSame(gmdate('Y-m-d\TH:i:s\Z', $expires), $response['body']['expiresAt']);
        self::assertSame('urn:kosmotheka:server', $claims['iss']);
        self::assertSame('http://mercure.test/.well-known/mercure', $claims['aud']);
    }

    public function testTheTokenIsSignedWithTheSubscriberKeyAndCannotPublish(): void
    {
        $this->uploadedLibrary($this->familySession, $this->laptop);
        $token = $this->tokenOf($this->call('GET', '/v1/cloud/events', $this->familySession)['body']);

        [$header, $payload, $signature] = explode('.', $token);
        $signed = $header.'.'.$payload;
        self::assertSame($signature, $this->base64Url(hash_hmac('sha256', $signed, $this->env('MERCURE_SUBSCRIBER_JWT_KEY'), true)));
        self::assertNotSame($signature, $this->base64Url(hash_hmac('sha256', $signed, $this->env('MERCURE_PUBLISHER_JWT_KEY'), true)));
        self::assertNotSame($this->env('MERCURE_PUBLISHER_JWT_KEY'), $this->env('MERCURE_SUBSCRIBER_JWT_KEY'));

        $claims = $this->claimsOf($token);
        self::assertArrayNotHasKey('mercure', $claims);
        foreach ($this->detailsOf($token) as $detail) {
            self::assertSame(['subscribe'], $detail['actions']);
        }
        self::assertStringNotContainsString('publish', json_encode($claims, \JSON_THROW_ON_ERROR));
    }

    public function testTheTokenRouteNeedsASignedInDevice(): void
    {
        $this->client->request('GET', '/v1/cloud/events');
        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);

        $this->client->request('GET', '/v1/cloud/events', server: ['HTTP_X-Device-Id' => $this->familySession['device_id'], 'HTTP_Authorization' => 'Bearer wrong']);
        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
    }

    public function testARemovedDeviceGetsNoToken(): void
    {
        $phone = $this->loginDevice($this->client, 'family@example.com', 'correct horse battery', Uuid::v4()->toRfc4122(), 'Phone');
        self::assertSame(Response::HTTP_OK, $this->call('GET', '/v1/cloud/events', $phone)['status']);

        $this->call('DELETE', '/v1/account/devices/'.$phone['device_id'], $this->familySession);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        $response = $this->call('GET', '/v1/cloud/events', $phone);
        self::assertSame(Response::HTTP_UNAUTHORIZED, $response['status']);
        self::assertSame(['error' => 'device_removed'], $response['body']);
    }

    public function testASignedOutDeviceGetsNoToken(): void
    {
        $phone = $this->loginDevice($this->client, 'family@example.com', 'correct horse battery', Uuid::v4()->toRfc4122(), 'Phone');
        $this->call('POST', '/v1/auth/logout', $phone);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        $response = $this->call('GET', '/v1/cloud/events', $phone);
        self::assertSame(Response::HTTP_UNAUTHORIZED, $response['status']);
    }

    // ---------------------------------------------------- what is published ---

    public function testAPushWithNewOperationsPublishesOnePrivateEmptyNoticeOnTheLibraryTopic(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $this->jhonnyHasALibrary();
        RecordingHub::reset();

        $response = $this->push($this->familySession, $cloud, $this->upload($this->laptop, $cloud));

        self::assertSame(4, $response['body']['received']);
        self::assertCount(1, RecordingHub::$updates);
        $update = RecordingHub::$updates[0];
        self::assertSame([self::TOPIC_PREFIX.'cloud-library:'.$cloud], $update->getTopics());
        self::assertTrue($update->isPrivate());
        self::assertSame('{}', $update->getData());
    }

    public function testAPushOfKnownOperationsPublishesNothing(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $operations = $this->upload($this->laptop, $cloud);
        $this->push($this->familySession, $cloud, $operations);
        RecordingHub::reset();

        $response = $this->push($this->familySession, $cloud, $operations);

        self::assertSame(['received' => 0, 'known' => 4, 'rejected' => 0], $response['body']);
        self::assertSame([], RecordingHub::$updates);
    }

    public function testAPushOfRejectedOperationsPublishesNothing(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        RecordingHub::reset();
        $unsigned = $this->laptop->publisher($this->laptop->libraryId(), self::T + 30);
        unset($unsigned['signature']);

        $response = $this->push($this->familySession, $cloud, [$unsigned]);

        self::assertSame(['received' => 0, 'known' => 0, 'rejected' => 1], $response['body']);
        self::assertSame([], RecordingHub::$updates);
    }

    public function testAnEmptyPushPublishesNothing(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        RecordingHub::reset();

        $this->push($this->familySession, $cloud, []);

        self::assertSame([], RecordingHub::$updates);
    }

    public function testAPushNobodyMayMakePublishesNothing(): void
    {
        $familyCloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        RecordingHub::reset();

        $response = $this->push($this->jhonnySession, $familyCloud, $this->upload($this->jhonny, $familyCloud));

        self::assertSame(Response::HTTP_NOT_FOUND, $response['status']);
        self::assertSame([], RecordingHub::$updates);
    }

    public function testAHubThatFailsDoesNotFailThePush(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        RecordingHub::$failing = true;

        $response = $this->push($this->familySession, $cloud, $this->upload($this->laptop, $cloud));

        self::assertSame(Response::HTTP_OK, $response['status']);
        self::assertSame(['received' => 4, 'known' => 0, 'rejected' => 0], $response['body']);
        self::assertSame(4, $this->rows('cloud_operation'));
    }

    public function testCreatingALibraryPublishesOnTheOwnersAccountTopicOnly(): void
    {
        $this->jhonnyHasALibrary();
        RecordingHub::reset();

        $this->createCloudLibrary($this->familySession);

        self::assertCount(1, RecordingHub::$updates);
        self::assertSame([self::TOPIC_PREFIX.'account:'.$this->userId('family@example.com')], RecordingHub::$updates[0]->getTopics());
        self::assertTrue(RecordingHub::$updates[0]->isPrivate());
        self::assertSame('{}', RecordingHub::$updates[0]->getData());
    }

    public function testAHubThatFailsDoesNotFailCreatingALibrary(): void
    {
        RecordingHub::$failing = true;

        $response = $this->call('POST', '/v1/cloud/libraries', $this->familySession);

        self::assertSame(Response::HTTP_CREATED, $response['status']);
        self::assertSame(1, $this->rows('cloud_library'));
    }

    public function testNoNoticeCarriesAnythingButTheEmptyObject(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $this->push($this->familySession, $cloud, [$this->laptop->publisher($this->laptop->libraryId(), self::T + 40, previous: $this->laptop->latest($this->laptop->libraryId()))]);

        self::assertNotSame([], RecordingHub::$updates);
        foreach (RecordingHub::$updates as $update) {
            self::assertSame('{}', $update->getData());
            self::assertTrue($update->isPrivate());
        }
    }

    // ------------------------------------------------------ Mercure off ---

    public function testWithoutMercureTheTokenRouteSaysEventsUnavailable(): void
    {
        $this->startWithoutHub();

        $response = $this->call('GET', '/v1/cloud/events', $this->familySession);

        self::assertSame(Response::HTTP_CONFLICT, $response['status']);
        self::assertSame(['error' => 'events_unavailable'], $response['body']);
    }

    public function testWithoutMercureTheServerSaysSoAndPushesStillWork(): void
    {
        $this->startWithoutHub();

        $this->client->request('GET', '/v1/server');
        self::assertFalse($this->serverEvents());

        $cloud = $this->createCloudLibrary($this->familySession);
        $response = $this->push($this->familySession, $cloud, $this->upload($this->laptop, $cloud));

        self::assertSame(['received' => 4, 'known' => 0, 'rejected' => 0], $response['body']);
        self::assertSame([], RecordingHub::$updates);
    }

    public function testWithMercureTheServerSaysSo(): void
    {
        $this->client->request('GET', '/v1/server');

        self::assertTrue($this->serverEvents());
    }

    // ------------------------------------------------------------ helpers ---

    private function start(string $prefix = ''): void
    {
        $this->client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();

        $this->familySession = $this->signInNewDevice($this->client, $prefix.'family@example.com');
        $this->jhonnySession = $this->signInNewDevice($this->client, $prefix.'jhonny@example.com');
        $this->laptop = SigningDevice::fromSession($this->familySession);
        $this->jhonny = SigningDevice::fromSession($this->jhonnySession);
    }

    /** The container reads the environment when it first builds a service, so the kernel is rebooted. */
    private function startWithoutHub(): void
    {
        self::ensureKernelShutdown();
        $this->setHub('');
        // The first start already took the emails, inside the test's transaction.
        $this->start('off.');
    }

    private function setHub(string $url): void
    {
        $_ENV['MERCURE_URL'] = $_SERVER['MERCURE_URL'] = $url;
    }

    private function jhonnyHasALibrary(): void
    {
        $this->createCloudLibrary($this->jhonnySession);
    }

    private function env(string $name): string
    {
        $value = $_SERVER[$name] ?? null;
        \assert(\is_string($value) && '' !== $value);

        return $value;
    }

    private function userId(string $email): string
    {
        $id = self::getContainer()->get(Connection::class)->fetchOne('SELECT id FROM "user" WHERE email = ?', [$email]);
        \assert(\is_string($id));

        return Uuid::fromString($id)->toRfc4122();
    }

    /** @param array<mixed> $body */
    private function tokenOf(array $body): string
    {
        $token = $body['token'] ?? null;
        \assert(\is_string($token));

        return $token;
    }

    private function serverEvents(): bool
    {
        $body = json_decode((string) $this->client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);
        \assert(\is_array($body) && \is_bool($body['events'] ?? null));

        return $body['events'];
    }

    /** @return array<string, mixed> */
    private function claimsOf(string $token): array
    {
        /** @var array<string, mixed> $claims */
        $claims = json_decode((string) base64_decode(strtr(explode('.', $token)[1], '-_', '+/'), true), true, flags: \JSON_THROW_ON_ERROR);

        return $claims;
    }

    /** @return list<array{actions: list<string>, topics: list<array{match: string, match_type?: string}>}> */
    private function detailsOf(string $token): array
    {
        /** @var list<array{actions: list<string>, topics: list<array{match: string, match_type?: string}>}> $details */
        $details = $this->claimsOf($token)['authorization_details'] ?? [];

        return $details;
    }

    /** @return list<string> */
    private function subscribedTopics(string $token): array
    {
        $topics = [];
        foreach ($this->detailsOf($token) as $detail) {
            self::assertContains('subscribe', $detail['actions']);
            foreach ($detail['topics'] as $topic) {
                self::assertArrayNotHasKey('match_type', $topic);
                $topics[] = $topic['match'];
            }
        }

        return $topics;
    }

    private function base64Url(string $bytes): string
    {
        return rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
    }

    /**
     * @param array{headers?: array<string, string>}|null $session
     * @param array<mixed>|null                           $body
     *
     * @return array{status: int, body: array<mixed>}
     */
    private function call(string $method, string $uri, ?array $session = null, ?array $body = null): array
    {
        $this->client->request(
            $method,
            $uri,
            server: ['CONTENT_TYPE' => 'application/json'] + ($session['headers'] ?? []),
            content: null === $body ? null : json_encode($body, \JSON_THROW_ON_ERROR),
        );
        $content = (string) $this->client->getResponse()->getContent();
        $decoded = '' === $content ? [] : json_decode($content, true, flags: \JSON_THROW_ON_ERROR);
        \assert(\is_array($decoded));

        return ['status' => $this->client->getResponse()->getStatusCode(), 'body' => $decoded];
    }

    /** @param array{headers: array<string, string>} $session */
    private function createCloudLibrary(array $session): string
    {
        $id = $this->call('POST', '/v1/cloud/libraries', $session)['body']['id'] ?? null;
        \assert(\is_string($id));

        return $id;
    }

    /** @return list<array<string, mixed>> */
    private function upload(SigningDevice $device, string $cloud): array
    {
        return [
            $device->founding(self::T, self::FOUNDER),
            $device->link($device->libraryId(), $cloud, self::T + 1),
            $device->publisher($device->libraryId(), self::T + 2),
            $device->publisher($device->libraryId(), self::T + 3),
        ];
    }

    /** @param array{headers: array<string, string>} $session */
    private function uploadedLibrary(array $session, SigningDevice $device): string
    {
        $cloud = $this->createCloudLibrary($session);
        self::assertSame(4, $this->push($session, $cloud, $this->upload($device, $cloud))['body']['received'] ?? null);

        return $cloud;
    }

    /**
     * @param array{headers: array<string, string>} $session
     * @param list<array<string, mixed>>            $operations
     *
     * @return array{status: int, body: array<mixed>}
     */
    private function push(array $session, string $cloud, array $operations): array
    {
        $version = Spec::json('sync.json')['schemaVersion'] ?? null;
        \assert(\is_int($version));

        return $this->call('POST', "/v1/cloud/libraries/$cloud/push", $session, [
            'schemaVersion' => $version,
            'operations' => array_map(static fn (array $operation): array => ['schemaVersion' => $version, 'operation' => $operation], $operations),
        ]);
    }

    private function rows(string $table): int
    {
        $count = self::getContainer()->get(Connection::class)->fetchOne("SELECT COUNT(*) FROM $table");
        \assert(\is_int($count));

        return $count;
    }
}
