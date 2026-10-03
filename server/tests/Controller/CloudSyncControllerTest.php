<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\DataFixtures\SigningDevice;
use App\Sync\OperationSignature;
use App\Tests\Support\DeviceAuthTestTrait;
use App\Tests\Support\Spec;
use Doctrine\DBAL\Connection;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\Uid\Uuid;

/**
 * The cloud routes of milestone 7.2 (docs/architecture/cloud.md) and the
 * «Nube» battery of docs/plans/fase-7.md. Every hostile case has the same
 * cast: a family, and Jhonny, who is hostile, has an account of his own and
 * knows the family's library, operation and device ids (D84).
 */
final class CloudSyncControllerTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    /** The fixed id of «Mi biblioteca» (D46), the same on every install. */
    private const string LOCAL = '00000000-0000-7000-8000-000000000001';
    private const int T = 1_727_000_000_000;

    private KernelBrowser $client;

    /** @var array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private array $familySession;

    /** @var array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private array $jhonnySession;

    private SigningDevice $laptop;
    private SigningDevice $jhonny;

    protected function setUp(): void
    {
        // The rate limiter cache is not rolled back with the database.
        $this->client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();

        $this->familySession = $this->signInNewDevice($this->client, 'family@example.com');
        $this->jhonnySession = $this->signInNewDevice($this->client, 'jhonny@example.com');
        $this->laptop = SigningDevice::fromSession($this->familySession);
        $this->jhonny = SigningDevice::fromSession($this->jhonnySession);
    }

    // ------------------------------------------------------------ normal ---

    public function testCreatingALibraryReturnsItsOwnId(): void
    {
        $response = $this->call('POST', '/v1/cloud/libraries', $this->familySession);

        self::assertSame(Response::HTTP_CREATED, $response['status']);
        self::assertIsString($response['body']['id'] ?? null);
        self::assertTrue(Uuid::isValid($response['body']['id']));
        self::assertSame(1, $this->rows('cloud_library'));
        self::assertSame(1, $this->rows('membership'));
    }

    public function testAccountsListOnlyTheirOwnLibraries(): void
    {
        $familyCloud = $this->createCloudLibrary($this->familySession);
        $this->createCloudLibrary($this->jhonnySession);
        $this->push($this->familySession, $familyCloud, $this->upload($this->laptop, $familyCloud));

        $response = $this->call('GET', '/v1/cloud/libraries', $this->familySession);

        self::assertSame(Response::HTTP_OK, $response['status']);
        self::assertSame([['id' => $familyCloud, 'localLibraryId' => self::LOCAL]], $response['body']);
    }

    public function testAFreshLibraryIsListedWithoutALocalId(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);

        $response = $this->call('GET', '/v1/cloud/libraries', $this->familySession);

        self::assertSame([['id' => $cloud, 'localLibraryId' => null]], $response['body']);
    }

    public function testUploadingALibraryLinksItAndStoresItsHistory(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $operations = $this->upload($this->laptop, $cloud);

        $response = $this->push($this->familySession, $cloud, $operations);

        self::assertSame(Response::HTTP_OK, $response['status']);
        self::assertSame(['received' => 4, 'known' => 0, 'rejected' => 0], $response['body']);
        self::assertSame(4, $this->rows('cloud_operation'));
        self::assertSame(1, $this->rows('cloud_device_key'));
    }

    public function testALinkLateInTheBatchIsProcessedFirst(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $operations = array_reverse($this->upload($this->laptop, $cloud));

        $response = $this->push($this->familySession, $cloud, $operations);

        self::assertSame(['received' => 4, 'known' => 0, 'rejected' => 0], $response['body']);
    }

    public function testPullingWithAnEmptyVectorReturnsEverythingInHlcOrder(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);

        $response = $this->pull($this->familySession, $cloud, []);

        self::assertSame(Response::HTTP_OK, $response['status']);
        $hlcs = $this->hlcsOf($response['body']);
        self::assertCount(4, $hlcs);
        $sorted = $hlcs;
        sort($sorted, \SORT_STRING);
        self::assertSame($sorted, $hlcs);
        self::assertSame([$this->laptop->id => end($hlcs)], $this->vectorOf($response['body']));
        $first = $this->operationsOf($response['body'])[0];
        self::assertSame($this->schemaVersion(), $first['schemaVersion']);
        // The operation comes back as it went in, signature included: it still verifies for anyone.
        // (JSONB does not keep key order; the canonical form does not need it.)
        $decoded = json_decode(json_encode($first['operation'], \JSON_THROW_ON_ERROR), false, flags: \JSON_THROW_ON_ERROR);
        self::assertInstanceOf(\stdClass::class, $decoded);
        new OperationSignature()->verify($decoded, $this->laptop->publicKey);
    }

    public function testPullingWithAPartialVectorReturnsOnlyWhatIsNewer(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $all = $this->hlcsOf($this->pull($this->familySession, $cloud, [])['body']);

        $response = $this->pull($this->familySession, $cloud, [$this->laptop->id => $all[1]]);

        self::assertSame(\array_slice($all, 2), $this->hlcsOf($response['body']));
    }

    public function testAnAuthorAbsentFromTheVectorComesInFull(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $phone = SigningDevice::generate();
        $this->push($this->familySession, $cloud, [$phone->registration(self::LOCAL, self::T + 10), $phone->publisher(self::LOCAL, self::T + 11)]);

        $response = $this->pull($this->familySession, $cloud, [$this->laptop->id => $this->hlcsOf($this->pull($this->familySession, $cloud, [])['body'])[3]]);

        self::assertCount(2, $this->operationsOf($response['body']));
        self::assertEqualsCanonicalizing([$this->laptop->id, $phone->id], array_keys($this->vectorOf($response['body'])));
    }

    public function testPushingTheSameOperationsTwiceChangesNothing(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $operations = $this->upload($this->laptop, $cloud);
        $this->push($this->familySession, $cloud, $operations);

        $response = $this->push($this->familySession, $cloud, $operations);

        self::assertSame(['received' => 0, 'known' => 4, 'rejected' => 0], $response['body']);
        self::assertSame(4, $this->rows('cloud_operation'));
    }

    public function testTheSameOperationTwiceInOneBatchIsStoredOnce(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $publisher = $this->laptop->publisher(self::LOCAL, self::T + 20);

        $response = $this->push($this->familySession, $cloud, [$publisher, $publisher]);

        self::assertSame(['received' => 1, 'known' => 1, 'rejected' => 0], $response['body']);
    }

    public function testADifferentSchemaVersionIsAConflictOnPullAndPush(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $other = $this->schemaVersion() + 1;

        $pull = $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $this->familySession, ['schemaVersion' => $other, 'vector' => []]);
        $push = $this->call('POST', "/v1/cloud/libraries/$cloud/push", $this->familySession, [
            'operations' => [['schemaVersion' => $other, 'operation' => $this->laptop->registration(self::LOCAL, self::T)]],
        ]);

        self::assertSame(Response::HTTP_CONFLICT, $pull['status']);
        self::assertSame(['error' => 'schema_version'], $pull['body']);
        self::assertSame(Response::HTTP_CONFLICT, $push['status']);
        self::assertSame(['error' => 'schema_version'], $push['body']);
        self::assertSame(0, $this->rows('cloud_operation'));
    }

    public function testOperationsWithABrokenStructureOrNoSignatureAreRejectedOneByOne(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $unsigned = $this->laptop->publisher(self::LOCAL, self::T + 30);
        unset($unsigned['signature']);
        $badIntent = $this->laptop->sign(self::LOCAL, self::T + 31, 'NOT AN INTENT', [['entity' => 'publisher', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['name' => 'x']]]);
        $tampered = $this->laptop->publisher(self::LOCAL, self::T + 32);
        $tampered['changes'] = [['entity' => 'publisher', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['name' => 'Somebody else']]];
        $good = $this->laptop->publisher(self::LOCAL, self::T + 33);

        $response = $this->push($this->familySession, $cloud, [$unsigned, $badIntent, $tampered, $good]);

        self::assertSame(['received' => 1, 'known' => 0, 'rejected' => 3], $response['body']);
    }

    public function testAnEmptyObjectOfFieldsSurvivesAndAListOfFieldsIsRejected(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $change = static fn (mixed $fields): array => [['entity' => 'publisher', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'delete', 'fields' => $fields]];
        $withObject = $this->laptop->sign(self::LOCAL, self::T + 40, 'publisher.remove', $change(new \stdClass()));
        $withList = $this->laptop->sign(self::LOCAL, self::T + 41, 'publisher.remove', $change([]));

        $response = $this->push($this->familySession, $cloud, [$withObject, $withList]);

        self::assertSame(['received' => 1, 'known' => 0, 'rejected' => 1], $response['body']);
        $this->client->request('POST', "/v1/cloud/libraries/$cloud/pull", server: ['CONTENT_TYPE' => 'application/json'] + $this->familySession['headers'], content: json_encode([
            'schemaVersion' => $this->schemaVersion(),
            'vector' => [$this->laptop->id => $this->hlcsOf($this->pull($this->familySession, $cloud, [])['body'])[3]],
        ], \JSON_THROW_ON_ERROR));
        $pulled = json_decode((string) $this->client->getResponse()->getContent(), false, flags: \JSON_THROW_ON_ERROR);
        \assert($pulled instanceof \stdClass && \is_array($pulled->operations) && 1 === \count($pulled->operations));
        $entry = $pulled->operations[0];
        \assert($entry instanceof \stdClass);
        $operation = $entry->operation;
        \assert($operation instanceof \stdClass && \is_array($operation->changes) && $operation->changes[0] instanceof \stdClass);
        self::assertInstanceOf(\stdClass::class, $operation->changes[0]->fields);
        new OperationSignature()->verify($operation, $this->laptop->publicKey);
    }

    public function testAMalformedBodyIsABadRequest(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);

        $response = $this->call('POST', "/v1/cloud/libraries/$cloud/push", $this->familySession, ['operations' => 'nope']);

        self::assertSame(Response::HTTP_BAD_REQUEST, $response['status']);
        self::assertSame(['error' => 'invalid_request'], $response['body']);
    }

    public function testTheCloudRoutesNeedASignedInDevice(): void
    {
        $this->client->request('GET', '/v1/cloud/libraries');

        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
    }

    public function testDeletingTheAccountDeletesItsCloudLibraries(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $this->call('DELETE', '/v1/account', $this->familySession, ['password' => 'correct horse battery']);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        self::assertSame(0, $this->rows('cloud_library'));
        self::assertSame(0, $this->rows('membership'));
        self::assertSame(0, $this->rows('cloud_operation'));
        self::assertSame(0, $this->rows('cloud_device_key'));
        self::assertNotSame('', $cloud);
    }

    // ------------------------------------------------- the «Nube» battery ---

    /** Battery 1. */
    public function testTheSameLocalIdInTwoLibrariesMakesTwoCloudLibraries(): void
    {
        $familyCloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $jhonnyCloud = $this->uploadedLibrary($this->jhonnySession, $this->jhonny);

        self::assertNotSame($familyCloud, $jhonnyCloud);

        $family = $this->hlcsOf($this->pull($this->familySession, $familyCloud, [])['body']);
        $theirs = $this->hlcsOf($this->pull($this->jhonnySession, $jhonnyCloud, [])['body']);
        self::assertCount(4, $family);
        self::assertCount(4, $theirs);
        self::assertSame([], array_intersect($family, $theirs));
        self::assertSame(8, $this->rows('cloud_operation'));
    }

    /** Battery 2. */
    public function testNoAccessIsTheSameAsNotExisting(): void
    {
        $familyCloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $before = $this->rows('cloud_operation');
        $missing = Uuid::v7()->toRfc4122();

        $pull = $this->pull($this->jhonnySession, $familyCloud, []);
        $push = $this->push($this->jhonnySession, $familyCloud, [$this->jhonny->registration(self::LOCAL, self::T + 50), $this->jhonny->link(self::LOCAL, $familyCloud, self::T + 51)]);
        $nothing = $this->pull($this->jhonnySession, $missing, []);
        $malformed = $this->pull($this->jhonnySession, 'not-a-uuid', []);

        foreach ([$pull, $push, $nothing, $malformed] as $response) {
            self::assertSame(Response::HTTP_NOT_FOUND, $response['status']);
            self::assertSame(['error' => 'not_found'], $response['body']);
        }
        self::assertSame($before, $this->rows('cloud_operation'));
        self::assertSame(1, $this->rows('cloud_device_key'));
        self::assertSame([], $this->call('GET', '/v1/cloud/libraries', $this->jhonnySession)['body']);
    }

    /** Battery 3. */
    public function testStolenOperationIdsDoNotKeepTheFamilyOut(): void
    {
        $familyCloud = $this->createCloudLibrary($this->familySession);
        $familyOperations = $this->upload($this->laptop, $familyCloud);
        $jhonnyCloud = $this->createCloudLibrary($this->jhonnySession);
        $stolen = $this->push($this->jhonnySession, $jhonnyCloud, [
            $this->jhonny->registration(self::LOCAL, self::T),
            $this->jhonny->link(self::LOCAL, $jhonnyCloud, self::T + 1),
            ...$familyOperations,
        ]);
        self::assertSame(['received' => 6, 'known' => 0, 'rejected' => 0], $stolen['body']);

        $response = $this->push($this->familySession, $familyCloud, $familyOperations);

        self::assertSame(['received' => 4, 'known' => 0, 'rejected' => 0], $response['body']);
        self::assertSame(10, $this->rows('cloud_operation'));
    }

    /** Battery 4. */
    public function testAnOperationOfAnotherLocalLibraryIsRejectedInBothAccounts(): void
    {
        $other = '00000000-0000-7000-8000-000000000002';
        $familyCloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $jhonnyCloud = $this->uploadedLibrary($this->jhonnySession, $this->jhonny);

        $family = $this->push($this->familySession, $familyCloud, [$this->laptop->publisher($other, self::T + 60)]);
        $theirs = $this->push($this->jhonnySession, $jhonnyCloud, [$this->jhonny->publisher($other, self::T + 60)]);

        self::assertSame(['received' => 0, 'known' => 0, 'rejected' => 1], $family['body']);
        self::assertSame(['received' => 0, 'known' => 0, 'rejected' => 1], $theirs['body']);
        self::assertSame(8, $this->rows('cloud_operation'));
    }

    /** Battery 5. */
    public function testWithoutALinkNothingEnters(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $jhonnyCloud = $this->createCloudLibrary($this->jhonnySession);

        $withoutLink = $this->push($this->familySession, $cloud, [
            $this->laptop->registration(self::LOCAL, self::T),
            $this->laptop->publisher(self::LOCAL, self::T + 1),
        ]);
        // A link for another cloud library, and a link signed by a device of another account.
        $forOther = $this->push($this->familySession, $cloud, [$this->laptop->link(self::LOCAL, $jhonnyCloud, self::T + 2)]);
        $foreign = $this->push($this->familySession, $cloud, [$this->jhonny->link(self::LOCAL, $cloud, self::T + 3)]);

        foreach ([$withoutLink['body'], $forOther['body'], $foreign['body']] as $body) {
            self::assertSame(0, $body['received']);
        }
        self::assertSame(2, $withoutLink['body']['rejected']);
        self::assertSame(1, $forOther['body']['rejected']);
        self::assertSame(1, $foreign['body']['rejected']);
        self::assertSame(0, $this->rows('cloud_operation'));
        self::assertSame([['id' => $cloud, 'localLibraryId' => null]], $this->call('GET', '/v1/cloud/libraries', $this->familySession)['body']);
    }

    /** Battery 6. */
    public function testSomebodyElsesKeyForADeviceIdIsRefused(): void
    {
        $phone = SigningDevice::generate();
        $impostor = SigningDevice::generate($phone->id);
        $familyCloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $jhonnyCloud = $this->uploadedLibrary($this->jhonnySession, $this->jhonny);

        // In his own cloud library Jhonny may register any id he likes.
        $registered = $this->push($this->jhonnySession, $jhonnyCloud, [$impostor->registration(self::LOCAL, self::T + 70)]);
        self::assertSame(1, $registered['body']['received']);

        $this->push($this->familySession, $familyCloud, [$phone->registration(self::LOCAL, self::T + 71)]);
        $forged = $this->push($this->familySession, $familyCloud, [
            $impostor->publisher(self::LOCAL, self::T + 72),
            $impostor->registration(self::LOCAL, self::T + 73),
        ]);
        $genuine = $this->push($this->familySession, $familyCloud, [$phone->publisher(self::LOCAL, self::T + 74)]);

        self::assertSame(['received' => 0, 'known' => 0, 'rejected' => 2], $forged['body']);
        self::assertSame(['received' => 1, 'known' => 0, 'rejected' => 0], $genuine['body']);
    }

    /** Battery 7. */
    public function testARemovedDeviceNoLongerPulls(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $phone = $this->loginDevice($this->client, 'family@example.com', 'correct horse battery', Uuid::v4()->toRfc4122(), 'Phone');
        self::assertSame(Response::HTTP_OK, $this->pull($phone, $cloud, [])['status']);

        $this->call('DELETE', '/v1/account/devices/'.$phone['device_id'], $this->familySession);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        foreach ([$this->pull($phone, $cloud, []), $this->push($phone, $cloud, [])] as $response) {
            self::assertSame(Response::HTTP_UNAUTHORIZED, $response['status']);
            self::assertSame(['error' => 'device_removed'], $response['body']);
        }
    }

    /** Battery 8 (DAT-001). */
    public function testAnOldOperationArrivingLateIsStoredNotRejected(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $this->push($this->familySession, $cloud, [
            $this->laptop->registration(self::LOCAL, self::T),
            $this->laptop->link(self::LOCAL, $cloud, self::T + 1),
            $this->laptop->publisher(self::LOCAL, self::T + 5, 'fifth'),
        ]);
        $fifth = $this->hlcsOf($this->pull($this->familySession, $cloud, [])['body'])[2];

        $late = $this->push($this->familySession, $cloud, [$this->laptop->publisher(self::LOCAL, self::T + 3, 'third')]);

        self::assertSame(['received' => 1, 'known' => 0, 'rejected' => 0], $late['body']);
        $partial = $this->pull($this->familySession, $cloud, [$this->laptop->id => $fifth]);
        self::assertSame([], $this->operationsOf($partial['body']));
        $everything = $this->hlcsOf($this->pull($this->familySession, $cloud, [])['body']);
        self::assertCount(4, $everything);
        self::assertStringStartsWith(\sprintf('%016d', self::T + 3), $everything[2]);
        self::assertStringStartsWith(\sprintf('%016d', self::T + 5), $everything[3]);
    }

    // ----------------------------------------------------------- helpers ---
    /**
     * @param array{headers: array<string, string>}|null $session
     * @param array<mixed>|null                          $body
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

    /**
     * @param array{headers: array<string, string>} $session
     */
    private function createCloudLibrary(array $session): string
    {
        $id = $this->call('POST', '/v1/cloud/libraries', $session)['body']['id'] ?? null;
        \assert(\is_string($id));

        return $id;
    }

    /**
     * Registration, link and two publishers: the history of a small library.
     *
     * @return list<array<string, mixed>>
     */
    private function upload(SigningDevice $device, string $cloud): array
    {
        return [
            $device->registration(self::LOCAL, self::T),
            $device->link(self::LOCAL, $cloud, self::T + 1),
            $device->publisher(self::LOCAL, self::T + 2),
            $device->publisher(self::LOCAL, self::T + 3),
        ];
    }

    /**
     * @param array{headers: array<string, string>} $session
     */
    private function uploadedLibrary(array $session, SigningDevice $device): string
    {
        $cloud = $this->createCloudLibrary($session);
        $response = $this->push($session, $cloud, $this->upload($device, $cloud));
        self::assertSame(4, $response['body']['received'] ?? null);

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
        $schemaVersion = $this->schemaVersion();

        return $this->call('POST', "/v1/cloud/libraries/$cloud/push", $session, [
            'operations' => array_map(static fn (array $operation): array => ['schemaVersion' => $schemaVersion, 'operation' => $operation], $operations),
        ]);
    }

    /**
     * @param array{headers: array<string, string>} $session
     * @param array<string, string>                 $vector
     *
     * @return array{status: int, body: array<mixed>}
     */
    private function pull(array $session, string $cloud, array $vector): array
    {
        return $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $session, [
            'schemaVersion' => $this->schemaVersion(),
            // An empty PHP array would be sent as a JSON list; the vector is an object.
            'vector' => [] === $vector ? new \stdClass() : $vector,
        ]);
    }

    /**
     * @param array<mixed> $body
     *
     * @return list<array{schemaVersion: int, operation: array<string, mixed>}>
     */
    private function operationsOf(array $body): array
    {
        /** @var list<array{schemaVersion: int, operation: array<string, mixed>}> $operations */
        $operations = $body['operations'] ?? [];

        return $operations;
    }

    /**
     * @param array<mixed> $body
     *
     * @return list<string>
     */
    private function hlcsOf(array $body): array
    {
        $hlcs = [];
        foreach ($this->operationsOf($body) as $entry) {
            \assert(\is_string($entry['operation']['hlc']));
            $hlcs[] = $entry['operation']['hlc'];
        }

        return $hlcs;
    }

    /**
     * @param array<mixed> $body
     *
     * @return array<string, string>
     */
    private function vectorOf(array $body): array
    {
        /** @var array<string, string> $vector */
        $vector = $body['vector'] ?? [];

        return $vector;
    }

    private function schemaVersion(): int
    {
        $version = Spec::json('sync.json')['schemaVersion'] ?? null;
        \assert(\is_int($version));

        return $version;
    }

    private function rows(string $table): int
    {
        $connection = self::getContainer()->get(Connection::class);

        $count = $connection->fetchOne("SELECT COUNT(*) FROM $table");
        \assert(\is_int($count));

        return $count;
    }
}
