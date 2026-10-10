<?php

declare(strict_types=1);

namespace App\Tests\Sync;

use App\DataFixtures\SigningDevice;
use App\Entity\CloudLibrary;
use App\Repository\CloudOperationRepository;
use App\Security\CloudKeyVault;
use App\Sync\CloudAuthor;
use App\Sync\CloudAuthorException;
use App\Sync\CloudDeviceId;
use App\Sync\CloudPush;
use App\Sync\OperationSignature;
use App\Sync\OperationSigner;
use App\Sync\SchemaVersion;
use App\Tests\Support\DeviceAuthTestTrait;
use Doctrine\DBAL\Connection;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\Clock\MockClock;
use Symfony\Component\Uid\Uuid;

/**
 * What the cloud signs itself (milestone 7.5.2a, D95): the operation goes through the path of a push, its signature
 * is the cloud device's, it chains to the cloud's previous one, and whether it has any power is the judge's call.
 * The cloud registers itself at the end of the push that names it.
 */
final class CloudAuthorTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string LOCAL = '00000000-0000-7000-8000-000000000001';
    private const int T = 1_727_000_000_000;
    private const string ADMIN = '01997c7e-0000-7000-8000-000000000011';
    private const string LAURA = '01997c7e-0000-7000-8000-000000000013';
    private const string ACCOUNT = '01997c7e-0000-7000-8000-0000000000a1';

    private KernelBrowser $client;

    /** @var array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private array $session;

    private SigningDevice $laptop;

    protected function setUp(): void
    {
        $this->client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();
        $this->session = $this->signInNewDevice($this->client, 'family@example.com');
        $this->laptop = SigningDevice::fromSession($this->session);
    }

    public function testTheCloudRegistersItselfAfterTheBatchThatNamesItAndOnlyOnce(): void
    {
        [$cloud, $identity] = $this->uploadedFamily(namesTheCloud: true);

        // The id derives from the key (D95).
        self::assertSame(CloudDeviceId::fromPublicKey($identity['key']), $identity['id']);
        $operations = $this->cloudOperations($cloud);
        self::assertCount(1, $operations);
        self::assertSame('applied', $operations[0]['status']);
        self::assertSame('device.register', $operations[0]['intent']);
        self::assertNull($operations[0]['previous']);
        self::assertSame($identity['id'], $operations[0]['author_device_id']);
        self::assertGreaterThan($this->hlcOf('library.link'), $operations[0]['hlc'], 'the newest in the log');
        self::assertNull($operations[0]['pushed_by_user_id']);
        $row = self::getContainer()->get(Connection::class)->fetchAssociative('SELECT public_key, name FROM catalog_devices WHERE id = ?', [$identity['id']]);
        self::assertSame($identity['key'], $row['public_key'] ?? null);

        // Another push does not write another one.
        $this->push($cloud, [$this->laptop->publisher(self::LOCAL, self::T + 20)]);
        self::assertCount(1, $this->cloudOperations($cloud));
    }

    public function testTheRegistrationIsWrittenWhateverTheWayTheLinkArrives(): void
    {
        $cloud = $this->createCloud();
        $identity = $this->identityOf($cloud);
        $operations = $this->family($cloud, $identity['id']);
        $this->call('POST', "/v1/cloud/libraries/$cloud/link", ['schemaVersion' => $this->schema(), 'operation' => $operations[2]]);

        // The batches arrive in HLC order: the link, which names the cloud, comes in the second.
        $first = $this->push($cloud, [$operations[0], $operations[1]]);
        self::assertSame(2, $first['received']);
        self::assertCount(0, $this->cloudOperations($cloud), 'not named yet');
        $this->push($cloud, [$operations[2], $operations[3]]);

        self::assertSame(['device.register'], array_column($this->cloudOperations($cloud), 'intent'));
    }

    public function testNothingIsWrittenWhenTheLibraryDoesNotNameItsCloud(): void
    {
        [$cloud] = $this->uploadedFamily(namesTheCloud: false);

        self::assertCount(0, $this->cloudOperations($cloud));
    }

    public function testTheCloudWritesAnAccountWhenTheLibraryNamesItAndItIsAppliedChainedAndSignedByItsKey(): void
    {
        [$cloud, $identity] = $this->uploadedFamily(namesTheCloud: true);

        $first = $this->authorAccount($cloud);
        $second = $this->authorAccount($cloud, null);

        $rows = $this->cloudOperations($cloud);
        self::assertCount(3, $rows);
        [$registration, $one, $two] = $rows;
        self::assertSame([$first, $second], [$one['operation_id'], $two['operation_id']]);
        self::assertSame(['applied', 'applied'], [$one['status'], $two['status']]);
        self::assertSame($registration['operation_id'], $one['previous']);
        self::assertSame($one['operation_id'], $two['previous']);
        self::assertLessThan($two['hlc'], $one['hlc']);
        self::assertNull($one['pushed_by_device_id']);
        $account = self::getContainer()->get(Connection::class)->fetchOne('SELECT account_id FROM catalog_members WHERE id = ?', [self::LAURA]);
        self::assertNull($account, 'the second operation emptied it');
        foreach ([$one, $two] as $row) {
            \assert(\is_string($row['body']));
            $body = json_decode($row['body'], false, flags: \JSON_THROW_ON_ERROR);
            \assert($body instanceof \stdClass);
            new OperationSignature()->verify($body, $identity['key']);
        }
        // The pull hands it out like any other.
        $pulled = $this->call('POST', "/v1/cloud/libraries/$cloud/pull", ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()]);
        \assert(\is_array($pulled['operations']));
        $ids = [];
        foreach ($pulled['operations'] as $entry) {
            \assert(\is_array($entry) && \is_array($entry['operation']));
            $ids[] = $entry['operation']['id'] ?? null;
        }
        self::assertContains($first, $ids);
    }

    public function testWithTheCloudNotNamedWhatItSignsIsARequestAndTheJudgeDecidesNotTheClass(): void
    {
        [$cloud] = $this->uploadedFamily(namesTheCloud: false);

        $id = $this->authorAccount($cloud);

        $status = self::getContainer()->get(Connection::class)->fetchOne('SELECT status FROM cloud_operation WHERE operation_id = ?', [$id]);
        self::assertSame('request', $status);
        self::assertNull(self::getContainer()->get(Connection::class)->fetchOne('SELECT account_id FROM catalog_members WHERE id = ?', [self::LAURA]));
    }

    public function testTwoOperationsWithAStoppedClockHaveDifferentHlcsInOrder(): void
    {
        [$cloud] = $this->uploadedFamily(namesTheCloud: true);
        $container = self::getContainer();
        $stopped = new CloudAuthor(
            $container->get(EntityManagerInterface::class),
            $container->get(CloudKeyVault::class),
            $container->get(CloudPush::class),
            $container->get(CloudOperationRepository::class),
            $container->get(OperationSigner::class),
            $container->get(SchemaVersion::class),
            new MockClock(new \DateTimeImmutable('@4102444800')), // 2100: ahead of any test time, and it does not move
            'http://kosmotheka.test',
        );

        $one = $this->authorWith($stopped, $cloud, self::ACCOUNT);
        $two = $this->authorWith($stopped, $cloud, null);

        $hlcs = $this->hlcsOf([$one, $two]);
        self::assertLessThan($hlcs[1], $hlcs[0]);
        self::assertNotSame($hlcs[0], $hlcs[1]);
        self::assertStringStartsWith('0004102444800000-00000000-', $hlcs[0]);
        self::assertStringStartsWith('0004102444800000-00000001-', $hlcs[1]);
    }

    public function testItNeedsATransactionAndALinkedLibrary(): void
    {
        $cloud = $this->createCloud();
        $em = self::getContainer()->get(EntityManagerInterface::class);
        $library = $em->find(CloudLibrary::class, Uuid::fromString($cloud)) ?? self::fail('No library.');
        $author = self::getContainer()->get(CloudAuthor::class);

        // Not in a transaction.
        try {
            $author->author($library, 'member.account', [$this->accountChange(self::ACCOUNT)]);
            self::fail('No transaction.');
        } catch (CloudAuthorException) {
        }
        // In one, but the library has no link yet.
        $this->expectException(CloudAuthorException::class);
        $em->wrapInTransaction(fn (): string => $author->author($library, 'member.account', [$this->accountChange(self::ACCOUNT)]));
    }

    // ----------------------------------------------------------- helpers ---
    /** @return array{0: string, 1: array{id: string, key: string}} */
    private function uploadedFamily(bool $namesTheCloud): array
    {
        $cloud = $this->createCloud();
        $identity = $this->identityOf($cloud);
        $response = $this->push($cloud, $this->family($cloud, $namesTheCloud ? $identity['id'] : null));
        self::assertSame(['received' => 4, 'known' => 0, 'rejected' => 0], $response);

        return [$cloud, $identity];
    }

    /**
     * The laptop's registration, the link (which names the cloud's device if asked) and the first roles: an
     * administrator, and Laura, an editor, who is only a row.
     *
     * @return list<array<string, mixed>> registration, library, link, roles
     */
    private function family(string $cloud, ?string $cloudDevice): array
    {
        $fields = ['cloud_library_id' => $cloud, 'cloud_server' => SigningDevice::SERVER] + (null === $cloudDevice ? [] : ['cloud_device_id' => $cloudDevice]);

        return [
            $this->laptop->registration(self::LOCAL, self::T),
            $this->laptop->sign(self::LOCAL, self::T + 1, 'library.create', [['entity' => 'library', 'id' => self::LOCAL, 'action' => 'create', 'fields' => ['name' => 'Family']]]),
            $this->laptop->sign(self::LOCAL, self::T + 2, 'library.link', [['entity' => 'library', 'id' => self::LOCAL, 'action' => 'update', 'fields' => $fields]]),
            $this->laptop->sign(self::LOCAL, self::T + 3, 'family.roles', [
                ['entity' => 'member', 'id' => self::ADMIN, 'action' => 'create', 'fields' => ['name' => 'Admin', 'role' => 'admin', 'can_share' => true]],
                ['entity' => 'member', 'id' => self::LAURA, 'action' => 'create', 'fields' => ['name' => 'Laura', 'role' => 'editor']],
                ['entity' => 'device', 'id' => $this->laptop->id, 'action' => 'update', 'fields' => ['member_id' => self::ADMIN]],
            ]),
        ];
    }

    private function authorAccount(string $cloud, ?string $account = self::ACCOUNT): string
    {
        return $this->authorWith(self::getContainer()->get(CloudAuthor::class), $cloud, $account);
    }

    private function authorWith(CloudAuthor $author, string $cloud, ?string $account): string
    {
        $em = self::getContainer()->get(EntityManagerInterface::class);
        $library = $em->find(CloudLibrary::class, Uuid::fromString($cloud)) ?? self::fail('No library.');

        return $em->wrapInTransaction(fn (): string => $author->author($library, 'member.account', [$this->accountChange($account)]));
    }

    /** @return array<string, mixed> */
    private function accountChange(?string $account): array
    {
        return ['entity' => 'member', 'id' => self::LAURA, 'action' => 'update', 'fields' => ['account_id' => $account]];
    }

    /** @return list<array<string, mixed>> */
    private function cloudOperations(string $cloud): array
    {
        $device = $this->identityOf($cloud)['id'];

        return self::getContainer()->get(Connection::class)->fetchAllAssociative(
            'SELECT operation_id, status, hlc, previous, author_device_id, pushed_by_user_id, pushed_by_device_id, CAST(body AS TEXT) AS body, body->>\'intent\' AS intent FROM cloud_operation WHERE cloud_library_id = ? AND author_device_id = ? ORDER BY hlc',
            [$cloud, $device],
        );
    }

    private function hlcOf(string $intent): string
    {
        $hlc = self::getContainer()->get(Connection::class)->fetchOne('SELECT hlc FROM cloud_operation WHERE body->>\'intent\' = ?', [$intent]);
        \assert(\is_string($hlc));

        return $hlc;
    }

    /**
     * @param list<string> $ids
     *
     * @return list<string>
     */
    private function hlcsOf(array $ids): array
    {
        return array_map(static function (string $id): string {
            $hlc = self::getContainer()->get(Connection::class)->fetchOne('SELECT hlc FROM cloud_operation WHERE operation_id = ?', [$id]);
            \assert(\is_string($hlc));

            return $hlc;
        }, $ids);
    }

    private function createCloud(): string
    {
        $id = $this->call('POST', '/v1/cloud/libraries')['id'] ?? null;
        \assert(\is_string($id));

        return $id;
    }

    /** @return array{id: string, key: string} */
    private function identityOf(string $cloud): array
    {
        foreach ($this->call('GET', '/v1/cloud/libraries') as $library) {
            if (\is_array($library) && ($library['id'] ?? null) === $cloud) {
                self::assertIsString($library['cloudDeviceId'] ?? null);
                self::assertIsString($library['cloudPublicKey'] ?? null);

                return ['id' => $library['cloudDeviceId'], 'key' => $library['cloudPublicKey']];
            }
        }

        self::fail('Not listed.');
    }

    /**
     * @param list<array<string, mixed>> $operations
     *
     * @return array<mixed>
     */
    private function push(string $cloud, array $operations): array
    {
        $version = $this->schema();

        return $this->call('POST', "/v1/cloud/libraries/$cloud/push", [
            'schemaVersion' => $version,
            'operations' => array_map(static fn (array $operation): array => ['schemaVersion' => $version, 'operation' => $operation], $operations),
        ]);
    }

    private function schema(): int
    {
        return self::getContainer()->get(SchemaVersion::class)->current();
    }

    /**
     * @param array<mixed>|null $body
     *
     * @return array<mixed>
     */
    private function call(string $method, string $uri, ?array $body = null): array
    {
        $this->client->request($method, $uri, server: ['CONTENT_TYPE' => 'application/json'] + $this->session['headers'], content: null === $body ? null : json_encode($body, \JSON_THROW_ON_ERROR));
        $decoded = json_decode((string) $this->client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);
        \assert(\is_array($decoded));

        return $decoded;
    }
}
