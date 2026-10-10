<?php

declare(strict_types=1);

namespace App\Tests\Sync;

use App\DataFixtures\SigningDevice;
use App\Entity\CloudLibrary;
use App\Entity\Membership;
use App\Entity\User;
use App\Sync\OperationSignature;
use App\Sync\SchemaVersion;
use App\Tests\Support\DeviceAuthTestTrait;
use Doctrine\DBAL\Connection;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\Uid\Uuid;

/**
 * `POST /v1/cloud/libraries/claim` (milestone 7.5.2b-2, D76, D96): the account of a device that is already a person's,
 * with a key somebody vouched for, gets that person's access. Jhonny is hostile: he has an account, knows the ids of
 * the family's devices and, where the case says so, has access to the library.
 */
final class AccountLinkerTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string LOCAL = '00000000-0000-7000-8000-000000000001';
    private const int T = 1_727_000_000_000;
    private const string LAURA = '01997c7e-0000-7000-8000-000000000011';
    private const string HUGO = '01997c7e-0000-7000-8000-000000000012';
    private const string READER = '01997c7e-0000-7000-8000-000000000013';
    private const string PASSWORD = 'correct horse battery';

    private KernelBrowser $client;
    private int $time = 100;

    /** @var array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private array $family;

    /** @var array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private array $jhonny;

    private SigningDevice $laptop;

    protected function setUp(): void
    {
        $this->client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();
        $this->family = $this->signInNewDevice($this->client, 'family@example.com');
        $this->jhonny = $this->signInNewDevice($this->client, 'jhonny@example.com');
        $this->laptop = SigningDevice::fromSession($this->family);
    }

    // ------------------------------------------------------ hostile ---

    /** D96: the id of Laura's new tablet registered first with Jhonny's key, then assigned to her with the real one. */
    public function testJhonnyWhoRegisteredTheTabletsIdWithAKeyOfHisOwnIsNotLinkedToLaura(): void
    {
        $cloud = $this->world();
        $this->shareWithJhonny($cloud);
        $tablet = SigningDevice::generate();
        $jhonnysTablet = $this->loginDevice($this->client, 'jhonny@example.com', self::PASSWORD, $tablet->id, 'Tablet');
        $impostor = SigningDevice::fromSession($jhonnysTablet);
        self::assertSame(1, $this->push($this->jhonny, $cloud, [$impostor->registration(self::LOCAL, self::T + $this->tick())])['received']);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($tablet->id, self::LAURA, $tablet->publicKey)]);

        $answer = $this->claim($jhonnysTablet);

        self::assertSame(['linked' => [], 'conflicts' => []], $answer);
        self::assertNull($this->memberOf($cloud, 'jhonny@example.com'), 'his row does not change');
        self::assertNull($this->accountIdOf(self::LAURA), 'nobody is Laura');
    }

    public function testJhonnyWithoutAccessAndAnIdOfLauraGetsNothingAndTheLibraryStaysNotFound(): void
    {
        $cloud = $this->world();
        // The id of Laura's laptop, with a key of his own.
        $mine = $this->loginDevice($this->client, 'jhonny@example.com', self::PASSWORD, $this->laptop->id, 'Laptop');

        $answer = $this->claim($mine);

        self::assertSame(['linked' => [], 'conflicts' => []], $answer);
        self::assertSame(404, $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $mine, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()])['status']);
        self::assertSame([], $this->call('GET', '/v1/cloud/libraries', $mine)['body']);
    }

    public function testAnAssignmentWithoutAVouchedKeyLinksNobody(): void
    {
        $cloud = $this->world();
        [$hugo, $session] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($hugo->id, self::HUGO)]);

        self::assertSame(['linked' => [], 'conflicts' => []], $this->claim($session));
        self::assertNull($this->memberOf($cloud, 'hugo@example.com'));
    }

    public function testARegisteredDeviceOfNobodyLinksNobody(): void
    {
        $cloud = $this->world();
        [, $session] = $this->hugo($cloud);

        self::assertSame(['linked' => [], 'conflicts' => []], $this->claim($session));
    }

    public function testAReaderWhoAssignsItselfToTheAdministratorVouchingForItsOwnKeyMakesARequestAndLinksNobody(): void
    {
        $cloud = $this->world();
        $this->shareWithJhonny($cloud);
        $mine = SigningDevice::fromSession($this->jhonny);
        $this->push($this->jhonny, $cloud, [
            $mine->registration(self::LOCAL, self::T + $this->tick()),
            $mine->sign(self::LOCAL, self::T + $this->tick(), 'device.claim', [$this->assign($mine->id, self::LAURA, $mine->publicKey)]),
        ]);

        self::assertSame(['linked' => [], 'conflicts' => []], $this->claim($this->jhonny));
        self::assertSame(1, $this->scalar("SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ? AND status = 'request'", [$cloud]));
        self::assertNull($this->memberOf($cloud, 'jhonny@example.com'));
    }

    public function testADeviceWithARevocationLinksNobody(): void
    {
        $cloud = $this->world();
        [$hugo, $session] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($hugo->id, self::HUGO, $hugo->publicKey)]);
        $this->laptopSigns($cloud, 'device.remove', [['entity' => 'revocation', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['device_id' => $hugo->id, 'device_name' => 'Hugo']]]);

        self::assertSame(['linked' => [], 'conflicts' => []], $this->claim($session));
        self::assertNull($this->memberOf($cloud, 'hugo@example.com'));
    }

    public function testAPersonWhoAlreadyHasAnotherAccountLinkedDoesNotLetASecondOneIn(): void
    {
        $cloud = $this->world();
        [$hugo, $session] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($hugo->id, self::HUGO, $hugo->publicKey)]);
        self::assertSame([$cloud], $this->claim($session)['linked']);
        // Jhonny's device, also assigned to Hugo's person with its true key.
        $other = SigningDevice::fromSession($this->jhonny);
        $this->push($this->family, $cloud, [$other->registration(self::LOCAL, self::T + $this->tick())]);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($other->id, self::HUGO, $other->publicKey)]);

        self::assertSame(['linked' => [], 'conflicts' => []], $this->claim($this->jhonny));
        self::assertNull($this->memberOf($cloud, 'jhonny@example.com'));
        self::assertSame(self::HUGO, $this->memberOf($cloud, 'hugo@example.com'));
        self::assertSame([], $this->call('GET', '/v1/cloud/libraries', $this->jhonny)['body']);
    }

    public function testABodyWithIdsIsIgnored(): void
    {
        $cloud = $this->world();
        $other = Uuid::v4()->toRfc4122();
        [$hugo, $session] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($hugo->id, self::HUGO, $hugo->publicKey)]);

        $answer = $this->call('POST', '/v1/cloud/libraries/claim', $session, ['library' => $other, 'member' => self::LAURA, 'device' => $this->laptop->id, 'libraries' => [$other]]);

        self::assertSame(200, $answer['status']);
        self::assertSame(['linked' => [$cloud], 'conflicts' => []], $answer['body']);
        self::assertSame(self::HUGO, $this->memberOf($cloud, 'hugo@example.com'));
        self::assertSame(1, $this->scalar('SELECT COUNT(*) FROM membership m JOIN "user" u ON u.id = m.user_id WHERE u.email = ?', ['hugo@example.com']));
    }

    public function testALibraryNotLinkedYetLinksNobody(): void
    {
        $cloud = $this->call('POST', '/v1/cloud/libraries', $this->family)['body']['id'] ?? null;
        \assert(\is_string($cloud));
        // Nothing but the link enters an unlinked library, whatever it says.
        $response = $this->push($this->family, $cloud, [
            $this->laptop->registration(self::LOCAL, self::T + $this->tick()),
            $this->laptop->sign(self::LOCAL, self::T + $this->tick(), 'family.roles', [
                ['entity' => 'member', 'id' => self::LAURA, 'action' => 'create', 'fields' => ['name' => 'Laura', 'role' => 'admin', 'can_share' => true]],
                $this->assign($this->laptop->id, self::LAURA, $this->laptop->publicKey),
            ]),
        ]);
        self::assertSame(0, $response['received']);

        self::assertSame(['linked' => [], 'conflicts' => []], $this->claim($this->family));
    }

    public function testAnAccountLinkedAsAReaderReadsAndWhatItUploadsIsARequest(): void
    {
        $cloud = $this->world();
        $readerSession = $this->signInNewDevice($this->client, 'reader2@example.com', self::PASSWORD, 'Reader');
        $device = SigningDevice::fromSession($readerSession);
        $this->push($this->family, $cloud, [$device->registration(self::LOCAL, self::T + $this->tick())]);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($device->id, self::READER, $device->publicKey)]);

        self::assertSame([$cloud], $this->claim($readerSession)['linked']);
        $pulled = $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $readerSession, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()]);
        self::assertSame(200, $pulled['status']);
        self::assertNotSame([], $pulled['body']['operations']);
        $before = $this->scalar("SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ? AND status = 'request'", [$cloud]);
        $this->push($readerSession, $cloud, [$device->publisher(self::LOCAL, self::T + $this->tick())]);
        self::assertSame($before + 1, $this->scalar("SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ? AND status = 'request'", [$cloud]), 'access gives no role');
    }

    // ------------------------------------------------------ the rest ---

    public function testAnEditorsAccountIsLinkedGetsTheLibraryAndTheCloudWritesItsAccount(): void
    {
        $cloud = $this->world();
        [$hugo, $session] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($hugo->id, self::HUGO, $hugo->publicKey)]);
        self::assertSame([], $this->call('GET', '/v1/cloud/libraries', $session)['body'], 'nothing before the claim');

        $answer = $this->claim($session);

        self::assertSame(['linked' => [$cloud], 'conflicts' => []], $answer);
        self::assertSame(self::HUGO, $this->memberOf($cloud, 'hugo@example.com'));
        $pulled = $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $session, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()]);
        self::assertSame(200, $pulled['status']);
        // The cloud wrote the account, applied, signed with its key, and the pull hands it out.
        $account = $this->accountIdOfUser('hugo@example.com');
        self::assertSame($account, $this->accountIdOf(self::HUGO));
        $rows = $this->cloudOperations($cloud, 'member.link_account');
        self::assertCount(1, $rows);
        self::assertSame('applied', $rows[0]['status']);
        $body = json_decode($rows[0]['body'], false, flags: \JSON_THROW_ON_ERROR);
        \assert($body instanceof \stdClass);
        new OperationSignature()->verify($body, $this->cloudKey($cloud));
        $pulledOperations = $pulled['body']['operations'] ?? null;
        \assert(\is_array($pulledOperations));
        $ids = array_map(static fn (mixed $entry): mixed => \is_array($entry) && \is_array($entry['operation'] ?? null) ? ($entry['operation']['id'] ?? null) : null, $pulledOperations);
        self::assertContains($rows[0]['operation_id'], $ids);
        // Nothing of the account but its public id travels.
        foreach ($this->allBodies($cloud) as $text) {
            self::assertStringNotContainsString('hugo@example.com', $text);
        }
    }

    public function testTheOwnerWhoUploadedHasHerRowFilledWithHerPerson(): void
    {
        $cloud = $this->world();
        self::assertNull($this->memberOf($cloud, 'family@example.com'), 'the owner starts with no person');

        self::assertSame(['linked' => [$cloud], 'conflicts' => []], $this->claim($this->family));

        self::assertSame(self::LAURA, $this->memberOf($cloud, 'family@example.com'));
        self::assertSame(1, $this->scalar('SELECT COUNT(*) FROM membership WHERE cloud_library_id = ? AND member_id IS NOT NULL', [$cloud]));
    }

    public function testTheCloudWithoutThePowerLinksAndWritesNothing(): void
    {
        $cloud = $this->world(namesTheCloud: false);
        [$hugo, $session] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($hugo->id, self::HUGO, $hugo->publicKey)]);

        self::assertSame([$cloud], $this->claim($session)['linked']);

        self::assertSame(self::HUGO, $this->memberOf($cloud, 'hugo@example.com'));
        self::assertNull($this->accountIdOf(self::HUGO));
        self::assertSame([], $this->cloudOperations($cloud, null));
    }

    public function testAnAccountThatIsAlreadyAnotherPersonGetsAConflictAndNothingChanges(): void
    {
        $cloud = $this->world();
        [$hugo, $session] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($hugo->id, self::READER, $hugo->publicKey)]);
        self::assertSame([$cloud], $this->claim($session)['linked']);
        // Hugo's second device is assigned to the other person.
        $secondSession = $this->loginDevice($this->client, 'hugo@example.com', self::PASSWORD, Uuid::v4()->toRfc4122(), 'Second');
        $second = SigningDevice::fromSession($secondSession);
        $this->push($this->family, $cloud, [$second->registration(self::LOCAL, self::T + $this->tick())]);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($second->id, self::HUGO, $second->publicKey)]);
        $operations = $this->scalar('SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ?', [$cloud]);

        self::assertSame(['linked' => [], 'conflicts' => [$cloud]], $this->claim($secondSession));

        self::assertSame(self::READER, $this->memberOf($cloud, 'hugo@example.com'));
        self::assertSame($operations, $this->scalar('SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ?', [$cloud]));
    }

    public function testClaimingTwiceChangesNothingTheSecondTime(): void
    {
        $cloud = $this->world();
        [$hugo, $session] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($hugo->id, self::HUGO, $hugo->publicKey)]);
        $this->claim($session);
        $operations = $this->scalar('SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ?', [$cloud]);

        self::assertSame(['linked' => [], 'conflicts' => []], $this->claim($session));

        self::assertSame($operations, $this->scalar('SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ?', [$cloud]));
        self::assertSame(1, $this->scalar('SELECT COUNT(*) FROM membership WHERE cloud_library_id = ? AND member_id = ?', [$cloud, self::HUGO]));
        self::assertCount(1, $this->cloudOperations($cloud, 'member.link_account'));
    }

    public function testARequestOfTheCloudIsNotWrittenAgainOnEveryClaim(): void
    {
        $cloud = $this->world();
        [$hugo, $session] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($hugo->id, self::HUGO, $hugo->publicKey)]);
        // The administrator takes the power away without telling the cloud's device it is no longer named: the
        // catalog still names it, but a fence makes it nobody's. Its write ends as a request, once.
        $this->claim($session);
        $this->forceRequest($cloud);
        $before = $this->scalar('SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ?', [$cloud]);
        $this->setAccount(self::HUGO, null);

        $this->claim($session);
        $this->claim($session);

        self::assertLessThanOrEqual($before + 1, $this->scalar('SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ?', [$cloud]));
    }

    public function testEveryAccountHasItsOwnPublicIdThatIsNotItsKey(): void
    {
        $ids = [];
        foreach (['family@example.com', 'jhonny@example.com'] as $email) {
            $row = self::getContainer()->get(Connection::class)->fetchAssociative('SELECT id, account_id FROM "user" WHERE email = ?', [$email]);
            \assert(\is_array($row));
            self::assertNotSame($row['id'], $row['account_id']);
            $account = $row['account_id'];
            \assert(\is_string($account));
            self::assertSame('4', Uuid::fromString($account)->toRfc4122()[14], 'a random v4');
            $ids[] = $account;
        }
        self::assertCount(2, array_unique($ids));
    }

    // ------------------------------------------------------ the world ---

    /**
     * A linked cloud library: the laptop (the family's account) is Laura's, an administrator, with its key vouched;
     * Hugo is an editor and the reader a reader, both only rows. The cloud device is named if asked.
     */
    private function world(bool $namesTheCloud = true): string
    {
        $cloud = $this->call('POST', '/v1/cloud/libraries', $this->family)['body']['id'] ?? null;
        \assert(\is_string($cloud));
        $identity = $this->identityOf($cloud);
        $fields = ['cloud_library_id' => $cloud, 'cloud_server' => SigningDevice::SERVER] + ($namesTheCloud ? ['cloud_device_id' => $identity] : []);
        $response = $this->push($this->family, $cloud, [
            $this->laptop->registration(self::LOCAL, self::T + $this->tick()),
            $this->laptop->sign(self::LOCAL, self::T + $this->tick(), 'library.create', [['entity' => 'library', 'id' => self::LOCAL, 'action' => 'create', 'fields' => ['name' => 'Family']]]),
            $this->laptop->sign(self::LOCAL, self::T + $this->tick(), 'library.link', [['entity' => 'library', 'id' => self::LOCAL, 'action' => 'update', 'fields' => $fields]]),
            $this->laptop->sign(self::LOCAL, self::T + $this->tick(), 'family.roles', [
                ['entity' => 'member', 'id' => self::LAURA, 'action' => 'create', 'fields' => ['name' => 'Laura', 'role' => 'admin', 'can_share' => true]],
                ['entity' => 'member', 'id' => self::HUGO, 'action' => 'create', 'fields' => ['name' => 'Hugo', 'role' => 'editor']],
                ['entity' => 'member', 'id' => self::READER, 'action' => 'create', 'fields' => ['name' => 'Reader', 'role' => 'reader']],
                $this->assign($this->laptop->id, self::LAURA, $this->laptop->publicKey),
            ]),
        ]);
        self::assertSame(4, $response['received']);

        return $cloud;
    }

    /**
     * Hugo's device, registered in the library (pushed by the family: whoever pushes, the signature is his).
     *
     * @return array{SigningDevice, array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string}}
     */
    private function hugo(string $cloud): array
    {
        $email = 'hugo@example.com';
        $session = null === $this->userId($email)
            ? $this->signInNewDevice($this->client, $email)
            : $this->loginDevice($this->client, $email, self::PASSWORD, Uuid::v4()->toRfc4122(), 'Hugo');
        $device = SigningDevice::fromSession($session);
        $this->push($this->family, $cloud, [$device->registration(self::LOCAL, self::T + $this->tick())]);

        return [$device, $session];
    }

    /**
     * @param list<array<string, mixed>> $changes
     */
    private function laptopSigns(string $cloud, string $intent, array $changes): void
    {
        $response = $this->push($this->family, $cloud, [$this->laptop->sign(self::LOCAL, self::T + $this->tick(), $intent, $changes)]);
        self::assertSame(1, $response['received']);
    }

    /** @return array<string, mixed> */
    private function assign(string $device, string $member, ?string $vouched = null): array
    {
        return ['entity' => 'device', 'id' => $device, 'action' => 'update', 'fields' => ['member_id' => $member] + (null === $vouched ? [] : ['vouched_key' => $vouched])];
    }

    private function tick(): int
    {
        return $this->time++;
    }

    // ------------------------------------------------------ the doors ---
    /**
     * @param array{headers: array<string, string>} $session
     *
     * @return array{linked: list<string>, conflicts: list<string>}
     */
    private function claim(array $session): array
    {
        $answer = $this->call('POST', '/v1/cloud/libraries/claim', $session);
        self::assertSame(200, $answer['status']);

        /** @var array{linked: list<string>, conflicts: list<string>} $body */
        $body = $answer['body'];

        return $body;
    }

    /**
     * @param array{headers: array<string, string>} $session
     * @param list<array<string, mixed>>            $operations
     *
     * @return array<mixed>
     */
    private function push(array $session, string $cloud, array $operations): array
    {
        $version = $this->schema();

        return $this->call('POST', "/v1/cloud/libraries/$cloud/push", $session, [
            'schemaVersion' => $version,
            'operations' => array_map(static fn (array $operation): array => ['schemaVersion' => $version, 'operation' => $operation], $operations),
        ])['body'];
    }

    /**
     * @param array{headers: array<string, string>}|null $session
     * @param array<mixed>|null                          $body
     *
     * @return array{status: int, body: array<mixed>}
     */
    private function call(string $method, string $uri, ?array $session = null, ?array $body = null): array
    {
        $this->client->request($method, $uri, server: ['CONTENT_TYPE' => 'application/json'] + ($session['headers'] ?? []), content: null === $body ? null : json_encode($body, \JSON_THROW_ON_ERROR));
        $content = (string) $this->client->getResponse()->getContent();
        $decoded = '' === $content ? [] : json_decode($content, true, flags: \JSON_THROW_ON_ERROR);
        \assert(\is_array($decoded));

        return ['status' => $this->client->getResponse()->getStatusCode(), 'body' => $decoded];
    }

    private function schema(): int
    {
        return self::getContainer()->get(SchemaVersion::class)->current();
    }

    private function shareWithJhonny(string $cloud): void
    {
        $em = self::getContainer()->get(EntityManagerInterface::class);
        $user = $em->getRepository(User::class)->findOneBy(['email' => 'jhonny@example.com']) ?? self::fail('No Jhonny.');
        $library = $em->find(CloudLibrary::class, Uuid::fromString($cloud)) ?? self::fail('No library.');
        $em->persist(new Membership($user, $library));
        $em->flush();
    }

    // ------------------------------------------------------ what the server holds ---

    /** The person the account has in the library, null if it has none or no access at all. */
    private function memberOf(string $cloud, string $email): ?string
    {
        $member = self::getContainer()->get(Connection::class)->fetchOne(
            'SELECT m.member_id FROM membership m JOIN "user" u ON u.id = m.user_id WHERE m.cloud_library_id = ? AND u.email = ?',
            [$cloud, $email],
        );

        return \is_string($member) ? $member : null;
    }

    private function accountIdOf(string $member): ?string
    {
        $account = self::getContainer()->get(Connection::class)->fetchOne('SELECT account_id FROM catalog_members WHERE id = ?', [$member]);

        return \is_string($account) ? $account : null;
    }

    private function accountIdOfUser(string $email): string
    {
        $account = self::getContainer()->get(Connection::class)->fetchOne('SELECT account_id FROM "user" WHERE email = ?', [$email]);
        \assert(\is_string($account));

        return $account;
    }

    private function userId(string $email): ?string
    {
        $id = self::getContainer()->get(Connection::class)->fetchOne('SELECT id FROM "user" WHERE email = ?', [$email]);

        return \is_string($id) ? $id : null;
    }

    /**
     * @param list<int|string> $params
     */
    private function scalar(string $sql, array $params = []): int
    {
        $count = self::getContainer()->get(Connection::class)->fetchOne($sql, $params);
        \assert(\is_int($count));

        return $count;
    }

    /** The cloud device's id in the library, from its owner's listing. */
    private function identityOf(string $cloud): string
    {
        foreach ($this->call('GET', '/v1/cloud/libraries', $this->family)['body'] as $library) {
            if (\is_array($library) && ($library['id'] ?? null) === $cloud && \is_string($library['cloudDeviceId'] ?? null)) {
                return $library['cloudDeviceId'];
            }
        }

        self::fail('Not listed.');
    }

    private function cloudKey(string $cloud): string
    {
        foreach ($this->call('GET', '/v1/cloud/libraries', $this->family)['body'] as $library) {
            if (\is_array($library) && ($library['id'] ?? null) === $cloud && \is_string($library['cloudPublicKey'] ?? null)) {
                return $library['cloudPublicKey'];
            }
        }

        self::fail('Not listed.');
    }

    /**
     * What the cloud device signed in the library (all of it, or one intent), its registration aside.
     *
     * @return list<array{operation_id: string, status: string, body: string}>
     */
    private function cloudOperations(string $cloud, ?string $intent): array
    {
        $rows = self::getContainer()->get(Connection::class)->fetchAllAssociative(
            'SELECT operation_id, status, body::text AS body FROM cloud_operation WHERE cloud_library_id = ? AND author_device_id = ? ORDER BY hlc',
            [$cloud, $this->identityOf($cloud)],
        );
        $found = [];
        foreach ($rows as $row) {
            \assert(\is_string($row['operation_id']) && \is_string($row['status']) && \is_string($row['body']));
            $operation = ['operation_id' => $row['operation_id'], 'status' => $row['status'], 'body' => $row['body']];
            if (str_contains($operation['body'], '"device.register"') || (null !== $intent && !str_contains($operation['body'], '"'.$intent.'"'))) {
                continue;
            }
            $found[] = $operation;
        }

        return $found;
    }

    /** @return list<string> */
    private function allBodies(string $cloud): array
    {
        $bodies = [];
        foreach (self::getContainer()->get(Connection::class)->fetchFirstColumn('SELECT body::text FROM cloud_operation WHERE cloud_library_id = ?', [$cloud]) as $body) {
            \assert(\is_string($body));
            $bodies[] = $body;
        }

        return $bodies;
    }

    /** Clears the catalog's account of a person, as if nothing had ever been written: only the loop test needs it. */
    private function setAccount(string $member, ?string $account): void
    {
        self::getContainer()->get(Connection::class)->executeStatement('UPDATE catalog_members SET account_id = ? WHERE id = ?', [$account, $member]);
    }

    /** Turns the cloud's operation about the person into a request, as a judge that took its power away would. */
    private function forceRequest(string $cloud): void
    {
        self::getContainer()->get(Connection::class)->executeStatement(
            "UPDATE cloud_operation SET status = 'request' WHERE cloud_library_id = ? AND body::text LIKE '%member.link_account%'",
            [$cloud],
        );
    }
}
