<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\DataFixtures\SigningDevice;
use App\Entity\CloudLibrary;
use App\Entity\Membership;
use App\Entity\User;
use App\Sync\OperationSignature;
use App\Tests\Support\DeviceAuthTestTrait;
use App\Tests\Support\Spec;
use Doctrine\DBAL\Connection;
use Doctrine\ORM\EntityManagerInterface;
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

    /**
     * The app uploads a library in batches of 1000 (PUSH_BATCH), in HLC order, and the link is the last operation:
     * the first batches arrive before the library is linked.
     */
    public function testALibraryUploadedInBatchesOf1000WithTheLinkLastIsStoredWhole(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $operations = $this->largeLibrary($cloud);
        self::assertCount(2_500, $operations);

        $linked = $this->link($this->familySession, $cloud, $operations[2_499]);
        $reports = array_map(fn (array $batch): array => $this->push($this->familySession, $cloud, $batch)['body'], array_chunk($operations, 1_000));

        self::assertSame(Response::HTTP_OK, $linked['status']);
        self::assertSame([
            ['received' => 1_000, 'known' => 0, 'rejected' => 0],
            ['received' => 1_000, 'known' => 0, 'rejected' => 0],
            ['received' => 500, 'known' => 0, 'rejected' => 0],
        ], $reports);
        self::assertSame(2_500, $this->rows('cloud_operation'));
    }

    // ------------------------------------------------ the link route (2b) ---

    public function testTheLinkRouteLinksTheLibraryWithoutStoringTheOperationAndRepeatingItChangesNothing(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $link = $this->laptop->link(self::LOCAL, $cloud, self::T + 1);

        $first = $this->link($this->familySession, $cloud, $link);
        $second = $this->link($this->familySession, $cloud, $link);

        self::assertSame(Response::HTTP_OK, $first['status']);
        self::assertSame(['localLibraryId' => self::LOCAL], $first['body']);
        self::assertSame($first, $second);
        self::assertSame(0, $this->rows('cloud_operation'));
        self::assertSame([['id' => $cloud, 'localLibraryId' => self::LOCAL]], $this->call('GET', '/v1/cloud/libraries', $this->familySession)['body']);
    }

    public function testALinkedLibraryRefusesALinkToAnotherLocalId(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $this->link($this->familySession, $cloud, $this->laptop->link(self::LOCAL, $cloud, self::T + 1));

        $other = $this->link($this->familySession, $cloud, $this->laptop->link('00000000-0000-7000-8000-000000000002', $cloud, self::T + 2));

        self::assertSame(['status' => Response::HTTP_UNPROCESSABLE_ENTITY, 'body' => ['error' => 'link_rejected']], $other);
        self::assertSame([['id' => $cloud, 'localLibraryId' => self::LOCAL]], $this->call('GET', '/v1/cloud/libraries', $this->familySession)['body']);
    }

    public function testSomeoneWithoutAccessCannotLinkTheFamilysLibrary(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);

        $response = $this->link($this->jhonnySession, $cloud, $this->jhonny->link(self::LOCAL, $cloud, self::T + 1));

        self::assertSame(['status' => Response::HTTP_NOT_FOUND, 'body' => ['error' => 'not_found']], $response);
        self::assertSame([['id' => $cloud, 'localLibraryId' => null]], $this->call('GET', '/v1/cloud/libraries', $this->familySession)['body']);
    }

    public function testSomeoneWithAccessWhoIsNotTheOwnerCannotLink(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $this->shareWithJhonny($cloud);

        $response = $this->link($this->jhonnySession, $cloud, $this->jhonny->link(self::LOCAL, $cloud, self::T + 1));

        self::assertSame(['status' => Response::HTTP_UNPROCESSABLE_ENTITY, 'body' => ['error' => 'link_rejected']], $response);
        self::assertSame([['id' => $cloud, 'localLibraryId' => null]], $this->call('GET', '/v1/cloud/libraries', $this->familySession)['body']);
    }

    public function testADeviceRemovedFromTheOwnersAccountCannotSignTheLink(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $phoneSession = $this->loginDevice($this->client, 'family@example.com', 'correct horse battery', Uuid::v4()->toRfc4122(), 'Phone');
        $phone = SigningDevice::fromSession($phoneSession);
        $this->call('DELETE', '/v1/account/devices/'.$phoneSession['device_id'], $this->familySession);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        $response = $this->link($this->familySession, $cloud, $phone->link(self::LOCAL, $cloud, self::T + 1));

        self::assertSame(['status' => Response::HTTP_UNPROCESSABLE_ENTITY, 'body' => ['error' => 'link_rejected']], $response);
        self::assertSame([['id' => $cloud, 'localLibraryId' => null]], $this->call('GET', '/v1/cloud/libraries', $this->familySession)['body']);
    }

    public function testALinkThatIsNotThisLibrarysIsRefused(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $jhonnyCloud = $this->createCloudLibrary($this->jhonnySession);
        $forgery = $this->laptop->link(self::LOCAL, $cloud, self::T + 1);
        $forgery['signature'] = 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA';
        $refused = [
            'another cloud library' => $this->laptop->link(self::LOCAL, $jhonnyCloud, self::T + 2),
            'not a link' => $this->laptop->publisher(self::LOCAL, self::T + 3),
            'a registration' => $this->laptop->registration(self::LOCAL, self::T + 4),
            'a bad signature' => $forgery,
        ];

        foreach ($refused as $what => $operation) {
            self::assertSame(['status' => Response::HTTP_UNPROCESSABLE_ENTITY, 'body' => ['error' => 'link_rejected']], $this->link($this->familySession, $cloud, $operation), $what);
        }
        self::assertSame([['id' => $cloud, 'localLibraryId' => null]], $this->call('GET', '/v1/cloud/libraries', $this->familySession)['body']);
    }

    public function testTheLinkRouteNeedsABodyOfTheRightShape(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);

        $response = $this->call('POST', "/v1/cloud/libraries/$cloud/link", $this->familySession, ['operation' => 'nope']);

        self::assertSame(['status' => Response::HTTP_BAD_REQUEST, 'body' => ['error' => 'invalid_request']], $response);
    }

    /**
     * A previous version of the app links with the batch that carries the link. With 1000 operations a batch
     * it uploads nothing the first time: this is what the link route is for.
     */
    public function testWithoutTheLinkRouteTheFirstPassOfALargeLibraryStoresNothingAndTheSecondStoresAll(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $operations = $this->largeLibrary($cloud);

        $firstPass = array_map(fn (array $batch): array => $this->push($this->familySession, $cloud, $batch)['body'], array_chunk($operations, 1_000));
        self::assertSame([['received' => 0, 'known' => 0, 'rejected' => 1_000], ['received' => 0, 'known' => 0, 'rejected' => 1_000], ['received' => 0, 'known' => 0, 'rejected' => 500]], $firstPass);
        self::assertSame(0, $this->rows('cloud_operation'));
        self::assertSame([['id' => $cloud, 'localLibraryId' => self::LOCAL]], $this->call('GET', '/v1/cloud/libraries', $this->familySession)['body']);

        $secondPass = array_map(fn (array $batch): array => $this->push($this->familySession, $cloud, $batch)['body'], array_chunk($operations, 1_000));
        self::assertSame([['received' => 1_000, 'known' => 0, 'rejected' => 0], ['received' => 1_000, 'known' => 0, 'rejected' => 0], ['received' => 500, 'known' => 0, 'rejected' => 0]], $secondPass);
        self::assertSame(2_500, $this->rows('cloud_operation'));
    }

    // ------------------------------------- who may upload a library (D93) ---

    private const string ADMIN_MEMBER = '00000000-0000-7000-8000-0000000000a1';
    private const string READER_MEMBER = '00000000-0000-7000-8000-0000000000a2';

    /**
     * The history of a family with roles: an administrator, a reader, and the administrator's device.
     *
     * @return array<string, mixed>
     */
    private function rolesOf(SigningDevice $admin, int $time = 0): array
    {
        return $admin->sign(self::LOCAL, self::T + $time, 'family.roles', [
            ['entity' => 'member', 'id' => self::ADMIN_MEMBER, 'action' => 'create', 'fields' => ['name' => 'Admin', 'role' => 'admin', 'can_share' => true]],
            ['entity' => 'member', 'id' => self::READER_MEMBER, 'action' => 'create', 'fields' => ['name' => 'Reader', 'role' => 'reader']],
            ['entity' => 'device', 'id' => $admin->id, 'action' => 'create', 'fields' => ['public_key' => $admin->publicKey, 'name' => 'Admin device', 'member_id' => self::ADMIN_MEMBER]],
        ]);
    }

    /**
     * The administrator says whose the reader's device is.
     *
     * @return array<string, mixed>
     */
    private function assignmentOf(SigningDevice $admin, SigningDevice $reader, int $time): array
    {
        return $admin->sign(self::LOCAL, self::T + $time, 'device.assign', [
            ['entity' => 'device', 'id' => $reader->id, 'action' => 'update', 'fields' => ['member_id' => self::READER_MEMBER]],
        ]);
    }

    private function assertNotLinkedAndNothingStored(string $cloud, int $operations = 0): void
    {
        self::assertSame($operations, $this->rows('cloud_operation'));
        self::assertSame([['id' => $cloud, 'localLibraryId' => null]], $this->call('GET', '/v1/cloud/libraries', $this->familySession)['body']);
    }

    public function testAReaderUploadingTheWholeLibraryWithItsRolesAndHisOwnLinkIsTurnedDown(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $admin = SigningDevice::generate();
        $operations = [
            $this->rolesOf($admin),
            $this->laptop->registration(self::LOCAL, self::T + 1),
            $this->assignmentOf($admin, $this->laptop, 2),
            $this->laptop->link(self::LOCAL, $cloud, self::T + 3),
        ];

        $response = $this->push($this->familySession, $cloud, $operations);

        self::assertSame(['status' => Response::HTTP_FORBIDDEN, 'body' => ['error' => 'link_not_allowed']], $response);
        $this->assertNotLinkedAndNothingStored($cloud);
        self::assertSame([], $this->statuses());
    }

    public function testAReaderWhoLinksByTheRouteStoresTheBatchesWithoutTheLinkAndNotTheOneWithIt(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $admin = SigningDevice::generate();
        $withoutLink = [$this->rolesOf($admin), $this->laptop->registration(self::LOCAL, self::T + 1), $this->assignmentOf($admin, $this->laptop, 2)];
        $link = $this->laptop->link(self::LOCAL, $cloud, self::T + 3);
        $later = $this->laptop->publisher(self::LOCAL, self::T + 4);

        self::assertSame(Response::HTTP_OK, $this->link($this->familySession, $cloud, $link)['status']);
        $first = $this->push($this->familySession, $cloud, $withoutLink);
        $second = $this->push($this->familySession, $cloud, [$later, $link]);

        self::assertSame(['received' => 3, 'known' => 0, 'rejected' => 0], $first['body']);
        self::assertSame(['status' => Response::HTTP_FORBIDDEN, 'body' => ['error' => 'link_not_allowed']], $second);
        self::assertSame(3, $this->rows('cloud_operation'));
        self::assertSame(['applied' => 3], $this->statuses());
    }

    public function testAReaderWhoUploadsTheCopyWithoutRolesFirstIsTurnedDownWhenTheRolesArrive(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $admin = SigningDevice::generate();
        $copy = $this->push($this->familySession, $cloud, [$this->laptop->registration(self::LOCAL, self::T + 1), $this->laptop->link(self::LOCAL, $cloud, self::T + 3)]);
        self::assertSame(['received' => 2, 'known' => 0, 'rejected' => 0], $copy['body']);
        self::assertSame(['applied' => 2], $this->statuses());

        $roles = $this->push($this->familySession, $cloud, [$this->rolesOf($admin), $this->assignmentOf($admin, $this->laptop, 2)]);

        self::assertSame(['status' => Response::HTTP_FORBIDDEN, 'body' => ['error' => 'link_not_allowed']], $roles);
        self::assertSame(2, $this->rows('cloud_operation'));
        self::assertSame(['applied' => 2], $this->statuses());
    }

    public function testAnAdministratorUploadingHerLibraryWithRolesLinksItAndHerLinkIsApplied(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $roles = $this->laptop->sign(self::LOCAL, self::T, 'family.roles', [
            ['entity' => 'member', 'id' => self::ADMIN_MEMBER, 'action' => 'create', 'fields' => ['name' => 'Admin', 'role' => 'admin', 'can_share' => true]],
            ['entity' => 'device', 'id' => $this->laptop->id, 'action' => 'create', 'fields' => ['public_key' => $this->laptop->publicKey, 'name' => 'Laptop', 'member_id' => self::ADMIN_MEMBER]],
        ]);

        $response = $this->push($this->familySession, $cloud, [$roles, $this->laptop->link(self::LOCAL, $cloud, self::T + 1)]);

        self::assertSame(['status' => Response::HTTP_OK, 'body' => ['received' => 2, 'known' => 0, 'rejected' => 0]], $response);
        self::assertSame(['applied' => 2], $this->statuses());
        self::assertSame([['id' => $cloud, 'localLibraryId' => self::LOCAL]], $this->call('GET', '/v1/cloud/libraries', $this->familySession)['body']);
    }

    /**
     * The link is the operation the server checked when it linked the library, not the first stored one that looks like it:
     * a reader with access who stores a link-shaped operation of his own before the owner's arrives must not block the library.
     */
    public function testAReadersLinkShapedOperationStoredBeforeTheOwnersLinkBlocksNothing(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $phoneSession = $this->loginDevice($this->client, 'family@example.com', 'correct horse battery', Uuid::v4()->toRfc4122(), 'Phone');
        $phone = SigningDevice::fromSession($phoneSession);
        $this->shareWithJhonny($cloud);
        $registration = $phone->registration(self::LOCAL, self::T);
        $ownersLink = $phone->link(self::LOCAL, $cloud, self::T + 1);
        $roles = $phone->sign(self::LOCAL, self::T + 2, 'family.roles', [
            ['entity' => 'member', 'id' => self::ADMIN_MEMBER, 'action' => 'create', 'fields' => ['name' => 'Admin', 'role' => 'admin', 'can_share' => true]],
            ['entity' => 'member', 'id' => self::READER_MEMBER, 'action' => 'create', 'fields' => ['name' => 'Reader', 'role' => 'reader']],
            ['entity' => 'device', 'id' => $phone->id, 'action' => 'update', 'fields' => ['member_id' => self::ADMIN_MEMBER]],
        ]);
        $assignment = $phone->sign(self::LOCAL, self::T + 6, 'device.assign', [['entity' => 'device', 'id' => $this->jhonny->id, 'action' => 'update', 'fields' => ['member_id' => self::READER_MEMBER]]]);
        self::assertSame(Response::HTTP_OK, $this->link($this->familySession, $cloud, $ownersLink)['status']);

        // The reader stores, first, an operation of his own that has the shape of a link.
        $readersRegistration = $this->jhonny->registration(self::LOCAL, self::T + 5);
        $readersLink = $this->jhonny->link(self::LOCAL, $cloud, self::T + 10);
        $first = $this->push($this->jhonnySession, $cloud, [$readersRegistration, $readersLink]);
        self::assertSame(['received' => 2, 'known' => 0, 'rejected' => 0], $first['body']);

        $owners = $this->push($this->familySession, $cloud, [$registration, $ownersLink, $roles, $assignment]);

        self::assertSame(['status' => Response::HTTP_OK, 'body' => ['received' => 4, 'known' => 0, 'rejected' => 0]], $owners);
        $statuses = self::getContainer()->get(Connection::class)->fetchAllKeyValue('SELECT operation_id, status FROM cloud_operation WHERE cloud_library_id = ?', [$cloud]);
        self::assertIsString($ownersLink['id']);
        self::assertIsString($readersLink['id']);
        self::assertSame('applied', $statuses[$ownersLink['id']]);
        self::assertSame('request', $statuses[$readersLink['id']]);
        // And the library still takes pushes.
        self::assertSame(Response::HTTP_OK, $this->push($this->familySession, $cloud, [$phone->publisher(self::LOCAL, self::T + 20)])['status']);
    }

    public function testAnyoneUploadsALibraryWithoutRoles(): void
    {
        $cloud = $this->createCloudLibrary($this->jhonnySession);

        $response = $this->push($this->jhonnySession, $cloud, $this->upload($this->jhonny, $cloud));

        self::assertSame(['status' => Response::HTTP_OK, 'body' => ['received' => 4, 'known' => 0, 'rejected' => 0]], $response);
        self::assertSame(['applied' => 4], $this->statuses());
    }

    // ------------------------------------------- Jhonny's battery, with roles ---
    /**
     * The family's cloud library with roles: its administrator is a second device of the account (the phone),
     * the laptop is a reader. Linked by the phone.
     *
     * @return array{string, SigningDevice} the cloud library id and the phone
     */
    private function familyWithRoles(): array
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $phoneSession = $this->loginDevice($this->client, 'family@example.com', 'correct horse battery', Uuid::v4()->toRfc4122(), 'Phone');
        $phone = SigningDevice::fromSession($phoneSession);
        $response = $this->push($this->familySession, $cloud, [
            $phone->registration(self::LOCAL, self::T),
            $phone->link(self::LOCAL, $cloud, self::T + 1),
            $phone->sign(self::LOCAL, self::T + 2, 'family.roles', [
                ['entity' => 'member', 'id' => self::ADMIN_MEMBER, 'action' => 'create', 'fields' => ['name' => 'Admin', 'role' => 'admin', 'can_share' => true]],
                ['entity' => 'member', 'id' => self::READER_MEMBER, 'action' => 'create', 'fields' => ['name' => 'Reader', 'role' => 'reader']],
                ['entity' => 'device', 'id' => $phone->id, 'action' => 'update', 'fields' => ['member_id' => self::ADMIN_MEMBER]],
            ]),
            $this->laptop->registration(self::LOCAL, self::T + 3),
            $phone->sign(self::LOCAL, self::T + 4, 'device.assign', [
                ['entity' => 'device', 'id' => $this->laptop->id, 'action' => 'update', 'fields' => ['member_id' => self::READER_MEMBER]],
            ]),
        ]);
        self::assertSame(['received' => 5, 'known' => 0, 'rejected' => 0], $response['body']);
        self::assertSame(['applied' => 5], $this->statuses());

        return [$cloud, $phone];
    }

    /**
     * Everything the cloud holds of one library that an operation could change.
     *
     * @return array<string, mixed>
     */
    private function snapshotOf(string $cloud): array
    {
        $connection = self::getContainer()->get(Connection::class);
        $snapshot = ['statuses' => $connection->fetchAllKeyValue('SELECT operation_id, status FROM cloud_operation WHERE cloud_library_id = ? ORDER BY operation_id', [$cloud])];
        foreach (['catalog_members' => 'id, role, name', 'catalog_devices' => 'id, member_key, name', 'catalog_publishers' => 'id, name'] as $table => $columns) {
            $snapshot[$table] = $connection->fetchAllAssociative("SELECT $columns FROM $table WHERE cloud_library_id = ? ORDER BY id", [$cloud]);
        }

        return $snapshot;
    }

    public function testJhonnyUploadsHisLibraryWithTheFamilysDeviceAndMemberIdsAndTheFamilysLibraryDoesNotChange(): void
    {
        [$familyCloud, $phone] = $this->familyWithRoles();
        $before = $this->snapshotOf($familyCloud);
        $jhonnyCloud = $this->createCloudLibrary($this->jhonnySession);

        // Everything he knows of the family's: ids of devices and members, the administrator's role among them.
        $response = $this->push($this->jhonnySession, $jhonnyCloud, [
            $this->jhonny->registration(self::LOCAL, self::T + 50),
            $this->jhonny->link(self::LOCAL, $jhonnyCloud, self::T + 51),
            $this->jhonny->sign(self::LOCAL, self::T + 52, 'forged.fence', [
                ['entity' => 'device', 'id' => $phone->id, 'action' => 'update', 'fields' => ['member_id' => self::READER_MEMBER, 'trusted_through' => Uuid::v7()->toRfc4122()]],
            ]),
            $this->jhonny->sign(self::LOCAL, self::T + 53, 'forged.member', [
                ['entity' => 'member', 'id' => self::ADMIN_MEMBER, 'action' => 'create', 'fields' => ['name' => 'Jhonny is admin', 'role' => 'admin', 'can_share' => true]],
            ]),
            $this->jhonny->sign(self::LOCAL, self::T + 54, 'forged.device', [
                ['entity' => 'device', 'id' => $this->laptop->id, 'action' => 'update', 'fields' => ['name' => 'stolen id', 'member_id' => self::ADMIN_MEMBER]],
            ]),
            $this->jhonny->publisher(self::LOCAL, self::T + 55, 'Jhonny publisher'),
        ]);

        self::assertSame(['received' => 6, 'known' => 0, 'rejected' => 0], $response['body']);
        self::assertSame($before, $this->snapshotOf($familyCloud));
        // His own library is judged by its own log: his forged member makes roles there, and his device is nobody's in them.
        self::assertSame(0, $this->scalarCount('SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ? AND status IS NULL', [$jhonnyCloud]));
        self::assertGreaterThan(0, $this->requestsIn($jhonnyCloud));
    }

    public function testAReaderOfTheFamilyUploadsAChangeSignedByHimItIsStoredAsARequestAndTheCatalogDoesNotChange(): void
    {
        [$cloud] = $this->familyWithRoles();
        $before = $this->snapshotOf($cloud);

        $response = $this->push($this->familySession, $cloud, [$this->laptop->publisher(self::LOCAL, self::T + 10, 'By a reader')]);

        self::assertSame(['received' => 1, 'known' => 0, 'rejected' => 0], $response['body']);
        $after = $this->snapshotOf($cloud);
        self::assertSame(['applied' => 5, 'request' => 1], $this->statuses());
        self::assertSame($before['catalog_publishers'], $after['catalog_publishers']);
        self::assertSame($before['catalog_members'], $after['catalog_members']);
        self::assertSame($before['catalog_devices'], $after['catalog_devices']);
    }

    /** SEG-014 through the route: the editor's backdated edit lands in the window of the fence of her demotion. */
    public function testADemotedEditorsBackdatedChangeIsARequest(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $phoneSession = $this->loginDevice($this->client, 'family@example.com', 'correct horse battery', Uuid::v4()->toRfc4122(), 'Phone');
        $phone = SigningDevice::fromSession($phoneSession);
        $editor = '00000000-0000-7000-8000-0000000000a3';
        $known = $this->laptop->registration(self::LOCAL, self::T + 3);
        $beforeTheDemotion = [
            $phone->registration(self::LOCAL, self::T),
            $phone->link(self::LOCAL, $cloud, self::T + 1),
            $phone->sign(self::LOCAL, self::T + 2, 'family.roles', [
                ['entity' => 'member', 'id' => self::ADMIN_MEMBER, 'action' => 'create', 'fields' => ['name' => 'Admin', 'role' => 'admin', 'can_share' => true]],
                ['entity' => 'member', 'id' => $editor, 'action' => 'create', 'fields' => ['name' => 'Editor', 'role' => 'editor']],
                ['entity' => 'device', 'id' => $phone->id, 'action' => 'update', 'fields' => ['member_id' => self::ADMIN_MEMBER]],
            ]),
            $known,
            $phone->sign(self::LOCAL, self::T + 4, 'device.assign', [['entity' => 'device', 'id' => $this->laptop->id, 'action' => 'update', 'fields' => ['member_id' => $editor]]]),
        ];
        $lastKnown = $this->laptop->publisher(self::LOCAL, self::T + 10, 'Known to the administrator');
        \assert(\is_string($lastKnown['id']));
        $demotion = $phone->sign(self::LOCAL, self::T + 20, 'member.role', [
            ['entity' => 'member', 'id' => $editor, 'action' => 'update', 'fields' => ['role' => 'reader']],
            ['entity' => 'device', 'id' => $this->laptop->id, 'action' => 'update', 'fields' => ['trusted_through' => $lastKnown['id']]],
        ]);
        $backdated = $this->laptop->publisher(self::LOCAL, self::T + 15, 'Backdated');
        self::assertSame(['received' => 7, 'known' => 0, 'rejected' => 0], $this->push($this->familySession, $cloud, [...$beforeTheDemotion, $lastKnown, $demotion])['body']);
        self::assertSame(['applied' => 7], $this->statuses());

        $response = $this->push($this->familySession, $cloud, [$backdated]);

        self::assertSame(['received' => 1, 'known' => 0, 'rejected' => 0], $response['body']);
        self::assertSame(['applied' => 7, 'request' => 1], $this->statuses());
        self::assertSame(1, $this->publishersIn($cloud));
    }

    /** 7.4.6: the log keeps who uploaded each operation, apart from who signed it. */
    public function testEachStoredOperationKeepsTheAccountAndDeviceThatUploadedIt(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $this->shareWithJhonny($cloud);
        $third = SigningDevice::generate();
        $registration = $third->registration(self::LOCAL, self::T + 10);
        $byFamily = $this->push($this->familySession, $cloud, [$registration]);
        $byJhonny = $this->push($this->jhonnySession, $cloud, [$third->publisher(self::LOCAL, self::T + 11, 'by Jhonny')]);
        self::assertSame(1, $byFamily['body']['received']);
        self::assertSame(1, $byJhonny['body']['received']);

        // The same operation again, from the other account: known, and it still counts for whoever sent it first.
        $again = $this->push($this->jhonnySession, $cloud, [$registration]);
        self::assertSame(1, $again['body']['known']);

        $stored = $this->uploaders($third->id);
        self::assertSame([
            [$this->userId('family@example.com'), $this->familySession['device_id']],
            [$this->userId('jhonny@example.com'), $this->jhonnySession['device_id']],
        ], $stored);
        self::assertSame([[$this->userId('family@example.com'), $this->familySession['device_id']]], array_values(array_unique($this->uploaders($this->laptop->id), \SORT_REGULAR)));
    }

    public function testALogRowKeepsItsDataWhenTheUploadersAccountIsDeleted(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $this->shareWithJhonny($cloud);
        $third = SigningDevice::generate();
        $this->push($this->jhonnySession, $cloud, [$third->registration(self::LOCAL, self::T + 10)]);

        $this->call('DELETE', '/v1/account', $this->jhonnySession, ['password' => 'correct horse battery']);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        self::assertSame([[null, $this->jhonnySession['device_id']]], $this->uploaders($third->id));
    }

    /** 7.4.6: what a push stores is judged and applied to the catalog, all in its transaction. */
    public function testAPushJudgesAndAppliesWhatItStores(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);

        self::assertSame(['applied' => 4], $this->statuses());
        self::assertSame(2, $this->rows('catalog_publishers'));
        self::assertSame(1, $this->rows('catalog_devices'));

        $this->push($this->familySession, $cloud, [$this->laptop->publisher(self::LOCAL, self::T + 10)]);
        self::assertSame(['applied' => 5], $this->statuses());
        self::assertSame(3, $this->rows('catalog_publishers'));
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
            'schemaVersion' => $other,
            'operations' => [['schemaVersion' => $other, 'operation' => $this->laptop->registration(self::LOCAL, self::T)]],
        ]);

        self::assertSame(Response::HTTP_CONFLICT, $pull['status']);
        self::assertSame(['error' => 'schema_version'], $pull['body']);
        self::assertSame(Response::HTTP_CONFLICT, $push['status']);
        self::assertSame(['error' => 'schema_version'], $push['body']);
        self::assertSame(0, $this->rows('cloud_operation'));
    }

    public function testAPushOfHistoryRecordedBeforeAnUpdateIsAccepted(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);

        $response = $this->push($this->familySession, $cloud, $this->upload($this->laptop, $cloud), operationVersion: $this->schemaVersion() - 1);

        self::assertSame(Response::HTTP_OK, $response['status']);
        self::assertSame(['received' => 4, 'known' => 0, 'rejected' => 0], $response['body']);
    }

    public function testAPushWhoseClientVersionDiffersIsAConflict(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);

        $response = $this->push($this->familySession, $cloud, $this->upload($this->laptop, $cloud), clientVersion: $this->schemaVersion() - 1);

        self::assertSame(Response::HTTP_CONFLICT, $response['status']);
        self::assertSame(['error' => 'schema_version'], $response['body']);
        self::assertSame(0, $this->rows('cloud_operation'));
    }

    public function testAPushWithoutItsClientVersionIsABadRequest(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);

        $response = $this->call('POST', "/v1/cloud/libraries/$cloud/push", $this->familySession, [
            'operations' => [['schemaVersion' => $this->schemaVersion(), 'operation' => $this->laptop->registration(self::LOCAL, self::T)]],
        ]);

        self::assertSame(Response::HTTP_BAD_REQUEST, $response['status']);
        self::assertSame(['error' => 'invalid_request'], $response['body']);
    }

    public function testAnOperationClaimingANewerVersionIsRejectedAlone(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $operations = $this->upload($this->laptop, $cloud);
        $schemaVersion = $this->schemaVersion();
        $this->client->request('POST', "/v1/cloud/libraries/$cloud/push", server: ['CONTENT_TYPE' => 'application/json'] + $this->familySession['headers'], content: json_encode([
            'schemaVersion' => $schemaVersion,
            'operations' => array_map(
                static fn (array $operation, int $version): array => ['schemaVersion' => $version, 'operation' => $operation],
                $operations,
                [$schemaVersion, $schemaVersion, $schemaVersion, $schemaVersion + 1],
            ),
        ], \JSON_THROW_ON_ERROR));
        $body = json_decode((string) $this->client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);

        self::assertResponseIsSuccessful();
        self::assertSame(['received' => 3, 'known' => 0, 'rejected' => 1], $body);
    }

    public function testARejectedOperationHoldsBackTheLaterOnesOfItsAuthor(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $stored = $this->vectorOf($this->pull($this->familySession, $cloud, [])['body']);
        // Both follow the last stored operation, so only the rejection holds the later one back, not the chain.
        $lastStored = $this->laptop->latest(self::LOCAL);
        $invalid = $this->laptop->sign(self::LOCAL, self::T + 10, 'publisher.add', [['entity' => 'publisher', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['name' => 'x', 'no_such_field' => 1]]]);
        $later = $this->laptop->publisher(self::LOCAL, self::T + 11, previous: $lastStored);

        $response = $this->push($this->familySession, $cloud, [$later, $invalid]);

        self::assertSame(['received' => 0, 'known' => 0, 'rejected' => 2], $response['body']);
        self::assertSame($stored, $this->vectorOf($this->pull($this->familySession, $cloud, [])['body']));

        $fixed = $this->laptop->publisher(self::LOCAL, self::T + 10, previous: $lastStored);
        $response = $this->push($this->familySession, $cloud, [$fixed, $later]);

        self::assertSame(['received' => 2, 'known' => 0, 'rejected' => 0], $response['body']);
    }

    public function testOperationsOfOtherAuthorsAreNotHeldBack(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $phone = SigningDevice::generate();
        $invalid = $this->laptop->sign(self::LOCAL, self::T + 10, 'publisher.add', [['entity' => 'publisher', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['name' => 'x', 'no_such_field' => 1]]]);

        $response = $this->push($this->familySession, $cloud, [$invalid, $phone->registration(self::LOCAL, self::T + 11)]);

        self::assertSame(['received' => 1, 'known' => 0, 'rejected' => 1], $response['body']);
    }

    public function testATextWithNulIsRejectedAndTheSixCharactersTypedAreNot(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $stored = $this->laptop->latest(self::LOCAL);
        $nul = $this->laptop->publisher(self::LOCAL, self::T + 10, "Minotauro\0");
        $typed = $this->laptop->publisher(self::LOCAL, self::T + 11, 'Minotauro\\u0000', $stored);

        $response = $this->push($this->familySession, $cloud, [$nul, $typed]);

        // The typed one is the author's later operation, so the rejection holds it back (DAT-001): sent alone it enters.
        self::assertSame(['received' => 0, 'known' => 0, 'rejected' => 2], $response['body']);
        $response = $this->push($this->familySession, $cloud, [$typed]);
        self::assertSame(['received' => 1, 'known' => 0, 'rejected' => 0], $response['body']);
    }

    public function testOperationsWithABrokenStructureOrNoSignatureAreRejectedOneByOne(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $stored = $this->laptop->latest(self::LOCAL);
        $unsigned = $this->laptop->publisher(self::LOCAL, self::T + 30);
        unset($unsigned['signature']);
        $badIntent = $this->laptop->sign(self::LOCAL, self::T + 31, 'NOT AN INTENT', [['entity' => 'publisher', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['name' => 'x']]]);
        $tampered = $this->laptop->publisher(self::LOCAL, self::T + 32);
        $tampered['changes'] = [['entity' => 'publisher', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['name' => 'Somebody else']]];
        // Before the others: a rejection holds back the author's later operations (DAT-001).
        $good = $this->laptop->publisher(self::LOCAL, self::T + 29, previous: $stored);

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

    /** D87: an operation whose previous the cloud lacks waits, and the author's vector stays below it. */
    public function testAnOperationWhoseCloudLacksThePreviousWaitsForIt(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $first = $this->laptop->publisher(self::LOCAL, self::T + 10, 'first');
        $second = $this->laptop->publisher(self::LOCAL, self::T + 11, 'second');
        $third = $this->laptop->publisher(self::LOCAL, self::T + 12, 'third');
        self::assertSame($first['id'], $second['previous']);

        $response = $this->push($this->familySession, $cloud, [$second, $third]);

        self::assertSame(['received' => 0, 'known' => 0, 'rejected' => 2], $response['body']);
        self::assertSame(4, $this->rows('cloud_operation'));
        $vector = $this->vectorOf($this->pull($this->familySession, $cloud, [])['body']);
        self::assertLessThan(\sprintf('%016d', self::T + 10), $vector[$this->laptop->id]);

        $response = $this->push($this->familySession, $cloud, [$third, $second, $first]);

        self::assertSame(['received' => 3, 'known' => 0, 'rejected' => 0], $response['body']);
        $vector = $this->vectorOf($this->pull($this->familySession, $cloud, [])['body']);
        self::assertStringStartsWith(\sprintf('%016d', self::T + 12), $vector[$this->laptop->id]);
    }

    /** D87: a previous that is another author's operation, or a later one, is a malformed chain. */
    public function testAPreviousThatIsNotAnEarlierOperationOfTheSameAuthorIsRefused(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $phone = SigningDevice::generate();
        $registration = $phone->registration(self::LOCAL, self::T + 20);
        \assert(\is_string($registration['id']));
        self::assertSame(['received' => 1, 'known' => 0, 'rejected' => 0], $this->push($this->familySession, $cloud, [$registration])['body']);
        $own = $this->laptop->publisher(self::LOCAL, self::T + 30, 'own');
        \assert(\is_string($own['id']));
        self::assertSame(['received' => 1, 'known' => 0, 'rejected' => 0], $this->push($this->familySession, $cloud, [$own])['body']);

        $ofAnotherAuthor = $this->laptop->sign(self::LOCAL, self::T + 31, 'publisher.add', [['entity' => 'publisher', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['name' => 'x']]], previous: $registration['id']);
        $notOlder = $this->laptop->sign(self::LOCAL, self::T + 30, 'publisher.add', [['entity' => 'publisher', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['name' => 'y']]], previous: $own['id']);
        $before = $this->rows('cloud_operation');

        self::assertSame(['received' => 0, 'known' => 0, 'rejected' => 1], $this->push($this->familySession, $cloud, [$ofAnotherAuthor])['body']);
        self::assertSame(['received' => 0, 'known' => 0, 'rejected' => 1], $this->push($this->familySession, $cloud, [$notOlder])['body']);
        self::assertSame($before, $this->rows('cloud_operation'));
        // A previous the push itself brings earlier is fine, as are two operations on the same previous (a fork).
        $honest = $this->laptop->sign(self::LOCAL, self::T + 32, 'publisher.add', [['entity' => 'publisher', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['name' => 'z']]], previous: $own['id']);
        self::assertSame(['received' => 1, 'known' => 0, 'rejected' => 0], $this->push($this->familySession, $cloud, [$honest])['body']);
    }

    /** Two different operations with one HLC (only a modified app signs them): the second is refused, and the push goes on. */
    public function testASecondOperationWithAnHlcAlreadyStoredIsRefused(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $own = $this->laptop->publisher(self::LOCAL, self::T + 30, 'own');
        \assert(\is_string($own['id']));
        self::assertSame(['received' => 1, 'known' => 0, 'rejected' => 0], $this->push($this->familySession, $cloud, [$own])['body']);
        $add = static fn (string $name): array => [['entity' => 'publisher', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['name' => $name]]];
        $first = $this->laptop->sign(self::LOCAL, self::T + 31, 'publisher.add', $add('first'), previous: $own['id']);
        $twin = $this->laptop->sign(self::LOCAL, self::T + 31, 'publisher.add', $add('twin'), previous: $own['id']);
        self::assertSame($first['hlc'], $twin['hlc']);
        $before = $this->rows('cloud_operation');

        self::assertSame(['received' => 1, 'known' => 0, 'rejected' => 1], $this->push($this->familySession, $cloud, [$first, $twin])['body']);
        self::assertSame(['received' => 0, 'known' => 0, 'rejected' => 1], $this->push($this->familySession, $cloud, [$twin])['body']);
        self::assertSame($before + 1, $this->rows('cloud_operation'));
    }

    /** D87: the key `previous` is required, even when it is null. */
    public function testAnOperationWithoutThePreviousKeyIsRejected(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $operation = $this->laptop->publisher(self::LOCAL, self::T + 10);
        unset($operation['previous'], $operation['signature']);
        $operation = $this->laptop->resign($operation);

        $response = $this->push($this->familySession, $cloud, [$operation]);

        self::assertSame(['received' => 0, 'known' => 0, 'rejected' => 1], $response['body']);
    }

    /** Battery 8 (DAT-001). Between authors: one author's chain does not wait for another's. */
    public function testAnOldOperationArrivingLateIsStoredNotRejected(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $phone = SigningDevice::generate();
        $this->push($this->familySession, $cloud, [
            $this->laptop->registration(self::LOCAL, self::T),
            $this->laptop->link(self::LOCAL, $cloud, self::T + 1),
            $phone->registration(self::LOCAL, self::T + 2),
            $phone->publisher(self::LOCAL, self::T + 5, 'fifth'),
        ]);
        $fifth = $this->hlcsOf($this->pull($this->familySession, $cloud, [])['body'])[3];

        $late = $this->push($this->familySession, $cloud, [$this->laptop->publisher(self::LOCAL, self::T + 3, 'third')]);

        self::assertSame(['received' => 1, 'known' => 0, 'rejected' => 0], $late['body']);
        $partial = $this->pull($this->familySession, $cloud, [$phone->id => $fifth, $this->laptop->id => \sprintf('%016d-00000000-%s', self::T + 3, $this->laptop->id)]);
        self::assertSame([], $this->operationsOf($partial['body']));
        $everything = $this->hlcsOf($this->pull($this->familySession, $cloud, [])['body']);
        self::assertCount(5, $everything);
        self::assertStringStartsWith(\sprintf('%016d', self::T + 3), $everything[3]);
        self::assertStringStartsWith(\sprintf('%016d', self::T + 5), $everything[4]);
    }

    /**
     * SEG-021: only an `update` of the library's own row links. A change of another row is already
     * refused by the structure rules (OtherLibrary); the action is what isLink must check itself.
     */
    public function testAnotherActionOnTheLibraryRowDoesNotLinkTheCloudLibrary(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $creation = $this->laptop->sign(self::LOCAL, self::T + 1, 'library.link', [[
            'entity' => 'library',
            'id' => self::LOCAL,
            'action' => 'create',
            'fields' => ['cloud_library_id' => $cloud, 'cloud_server' => SigningDevice::SERVER],
        ]]);

        $response = $this->push($this->familySession, $cloud, [$this->laptop->registration(self::LOCAL, self::T), $creation]);

        self::assertSame(['received' => 0, 'known' => 0, 'rejected' => 2], $response['body']);
        self::assertSame(0, $this->rows('cloud_operation'));
        self::assertSame([['id' => $cloud, 'localLibraryId' => null]], $this->call('GET', '/v1/cloud/libraries', $this->familySession)['body']);
    }

    /** SEG-021: a change of another library row never reaches isLink; the structure rules refuse it. */
    public function testALinkOnAnotherLibraryRowDoesNotLinkTheCloudLibrary(): void
    {
        $cloud = $this->createCloudLibrary($this->familySession);
        $misdirected = $this->laptop->sign(self::LOCAL, self::T + 1, 'library.link', [[
            'entity' => 'library',
            'id' => '00000000-0000-7000-8000-000000000002',
            'action' => 'update',
            'fields' => ['cloud_library_id' => $cloud, 'cloud_server' => SigningDevice::SERVER],
        ]]);

        $response = $this->push($this->familySession, $cloud, [$this->laptop->registration(self::LOCAL, self::T), $misdirected]);

        self::assertSame(['received' => 0, 'known' => 0, 'rejected' => 2], $response['body']);
        self::assertSame(0, $this->rows('cloud_operation'));
        self::assertSame([['id' => $cloud, 'localLibraryId' => null]], $this->call('GET', '/v1/cloud/libraries', $this->familySession)['body']);
    }

    /** SEG-019 (part 1): a registered device changing its own key. */
    public function testARegisteredDeviceCannotChangeItsOwnKey(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $other = SigningDevice::generate();

        $response = $this->push($this->familySession, $cloud, [$this->deviceKeyChange($this->laptop, $this->laptop->id, $other->publicKey, self::T + 80)]);

        self::assertSame(['received' => 0, 'known' => 0, 'rejected' => 1], $response['body']);
        self::assertSame([$this->laptop->publicKey], $this->storedKeys($this->laptop->id));
    }

    /** SEG-019 (part 1): a device changing the key of another registered device. */
    public function testADeviceCannotChangeTheKeyOfAnotherRegisteredDevice(): void
    {
        $cloud = $this->uploadedLibrary($this->familySession, $this->laptop);
        $phone = SigningDevice::generate();
        $this->push($this->familySession, $cloud, [$phone->registration(self::LOCAL, self::T + 81)]);
        $other = SigningDevice::generate();

        $response = $this->push($this->familySession, $cloud, [$this->deviceKeyChange($this->laptop, $phone->id, $other->publicKey, self::T + 82)]);

        self::assertSame(['received' => 0, 'known' => 0, 'rejected' => 1], $response['body']);
        self::assertSame([$phone->publicKey], $this->storedKeys($phone->id));
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
     * An operation signed by $author that sets $key on the device row $target.
     *
     * @return array<string, mixed>
     */
    private function deviceKeyChange(SigningDevice $author, string $target, string $key, int $time): array
    {
        return $author->sign(self::LOCAL, $time, 'device.rekey', [[
            'entity' => 'device',
            'id' => $target,
            'action' => 'update',
            'fields' => ['public_key' => $key],
        ]]);
    }

    /**
     * @return list<string>
     */
    private function storedKeys(string $device): array
    {
        $keys = self::getContainer()->get(Connection::class)->fetchFirstColumn('SELECT public_key FROM cloud_device_key WHERE device_id = ?', [$device]);

        return array_map(static fn (mixed $key): string => \is_string($key) ? $key : '', $keys);
    }

    /**
     * A registration, 2498 publishers and the link, in HLC order: what the app uploads, link last.
     *
     * @return list<array<string, mixed>>
     */
    private function largeLibrary(string $cloud): array
    {
        $operations = [$this->laptop->registration(self::LOCAL, self::T)];
        for ($n = 1; $n < 2_499; ++$n) {
            $operations[] = $this->laptop->publisher(self::LOCAL, self::T + $n, "Editorial $n");
        }
        $operations[] = $this->laptop->link(self::LOCAL, $cloud, self::T + 2_499);

        return $operations;
    }

    /**
     * @param array{headers: array<string, string>} $session
     * @param array<string, mixed>                  $operation
     *
     * @return array{status: int, body: array<mixed>}
     */
    private function link(array $session, string $cloud, array $operation): array
    {
        return $this->call('POST', "/v1/cloud/libraries/$cloud/link", $session, ['schemaVersion' => $this->schemaVersion(), 'operation' => $operation]);
    }

    /** Gives Jhonny's account access to the family's cloud library, as the sharing of 7.5 will. */
    private function shareWithJhonny(string $cloud): void
    {
        $em = self::getContainer()->get(EntityManagerInterface::class);
        $user = $em->getRepository(User::class)->findOneBy(['email' => 'jhonny@example.com']) ?? self::fail('No Jhonny.');
        $library = $em->find(CloudLibrary::class, Uuid::fromString($cloud)) ?? self::fail('No library.');
        $em->persist(new Membership($user, $library));
        $em->flush();
    }

    /**
     * How many operations of the log ended in each state; `none` for those not judged.
     *
     * @return array<string, int>
     */
    private function statuses(): array
    {
        $counts = [];
        $rows = self::getContainer()->get(Connection::class)->fetchAllNumeric('SELECT COALESCE(status, \'none\'), COUNT(*) FROM cloud_operation GROUP BY 1 ORDER BY 1');
        foreach ($rows as [$status, $count]) {
            \assert(\is_string($status) && \is_int($count));
            $counts[$status] = $count;
        }

        return $counts;
    }

    /**
     * @param list<string> $params
     */
    private function scalarCount(string $sql, array $params = []): int
    {
        $count = self::getContainer()->get(Connection::class)->fetchOne($sql, $params);
        \assert(\is_int($count));

        return $count;
    }

    private function requestsIn(string $cloud): int
    {
        return $this->scalarCount("SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ? AND status = 'request'", [$cloud]);
    }

    private function publishersIn(string $cloud): int
    {
        return $this->scalarCount('SELECT COUNT(*) FROM catalog_publishers WHERE cloud_library_id = ?', [$cloud]);
    }

    private function userId(string $email): string
    {
        $id = self::getContainer()->get(Connection::class)->fetchOne('SELECT id FROM "user" WHERE email = ?', [$email]);
        \assert(\is_string($id));

        return $id;
    }

    /**
     * Who uploaded each operation signed by a device, in HLC order.
     *
     * @return list<array{0: ?string, 1: ?string}> user id and device id
     */
    private function uploaders(string $author): array
    {
        $rows = self::getContainer()->get(Connection::class)->fetchAllNumeric('SELECT pushed_by_user_id, pushed_by_device_id FROM cloud_operation WHERE author_device_id = ? ORDER BY hlc', [$author]);

        return array_map(static fn (array $row): array => [\is_string($row[0]) ? $row[0] : null, \is_string($row[1]) ? $row[1] : null], $rows);
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
     * @param int|null                              $clientVersion    the push's own `schemaVersion`; the server's when null
     * @param int|null                              $operationVersion the one each operation was recorded at; the server's when null
     *
     * @return array{status: int, body: array<mixed>}
     */
    private function push(array $session, string $cloud, array $operations, ?int $clientVersion = null, ?int $operationVersion = null): array
    {
        $operationVersion ??= $this->schemaVersion();

        return $this->call('POST', "/v1/cloud/libraries/$cloud/push", $session, [
            'schemaVersion' => $clientVersion ?? $this->schemaVersion(),
            'operations' => array_map(static fn (array $operation): array => ['schemaVersion' => $operationVersion, 'operation' => $operation], $operations),
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
