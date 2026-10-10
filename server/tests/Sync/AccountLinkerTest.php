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

        self::assertSame(['linked' => [], 'conflicts' => [], 'devices' => []], $answer);
        self::assertSame(self::READER, $this->memberOf($cloud, 'jhonny@example.com'), 'his row does not change');
        self::assertNull($this->accountIdOf(self::LAURA), 'nobody is Laura');
    }

    public function testJhonnyWithoutAccessAndAnIdOfLauraGetsNothingAndTheLibraryStaysNotFound(): void
    {
        $cloud = $this->world();
        // The id of Laura's laptop, with a key of his own.
        $mine = $this->loginDevice($this->client, 'jhonny@example.com', self::PASSWORD, $this->laptop->id, 'Laptop');

        $answer = $this->claim($mine);

        self::assertSame(['linked' => [], 'conflicts' => [], 'devices' => []], $answer);
        self::assertSame(404, $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $mine, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()])['status']);
        self::assertSame([], $this->libraryList($this->call('GET', '/v1/cloud/libraries', $mine)['body']));
    }

    public function testAnAssignmentWithoutAVouchedKeyLinksNobody(): void
    {
        $cloud = $this->world();
        [$hugo, $session] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($hugo->id, self::HUGO)]);

        self::assertSame(['linked' => [], 'conflicts' => [], 'devices' => []], $this->claim($session));
        self::assertNull($this->memberOf($cloud, 'hugo@example.com'));
    }

    public function testARegisteredDeviceOfNobodyLinksNobody(): void
    {
        $cloud = $this->world();
        [, $session] = $this->hugo($cloud);

        self::assertSame(['linked' => [], 'conflicts' => [], 'devices' => []], $this->claim($session));
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

        // His account is the reader's, so the cloud says his device is the reader's: never Laura's, whatever he signed.
        self::assertSame(['linked' => [], 'conflicts' => [], 'devices' => [$cloud]], $this->claim($this->jhonny));
        self::assertSame(self::READER, $this->personOfDevice($mine->id));
        self::assertNull($this->accountIdOf(self::LAURA));
        self::assertSame(1, $this->scalar("SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ? AND status = 'request'", [$cloud]));
        self::assertSame(self::READER, $this->memberOf($cloud, 'jhonny@example.com'));
    }

    public function testADeviceWithARevocationLinksNobody(): void
    {
        $cloud = $this->world();
        [$hugo, $session] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($hugo->id, self::HUGO, $hugo->publicKey)]);
        $this->laptopSigns($cloud, 'device.remove', [['entity' => 'revocation', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['device_id' => $hugo->id, 'device_name' => 'Hugo']]]);

        self::assertSame(['linked' => [], 'conflicts' => [], 'devices' => []], $this->claim($session));
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

        self::assertSame(['linked' => [], 'conflicts' => [], 'devices' => []], $this->claim($this->jhonny));
        self::assertNull($this->memberOf($cloud, 'jhonny@example.com'));
        self::assertSame(self::HUGO, $this->memberOf($cloud, 'hugo@example.com'));
        self::assertSame([], $this->libraryList($this->call('GET', '/v1/cloud/libraries', $this->jhonny)['body']));
    }

    public function testABodyWithIdsIsIgnored(): void
    {
        $cloud = $this->world();
        $other = Uuid::v4()->toRfc4122();
        [$hugo, $session] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($hugo->id, self::HUGO, $hugo->publicKey)]);

        $answer = $this->call('POST', '/v1/cloud/libraries/claim', $session, ['library' => $other, 'member' => self::LAURA, 'device' => $this->laptop->id, 'libraries' => [$other]]);

        self::assertSame(200, $answer['status']);
        self::assertSame(['linked' => [$cloud], 'conflicts' => [], 'devices' => []], $answer['body']);
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

        self::assertSame(['linked' => [], 'conflicts' => [], 'devices' => []], $this->claim($this->family));
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
        self::assertSame([], $this->libraryList($this->call('GET', '/v1/cloud/libraries', $session)['body']), 'nothing before the claim');

        $answer = $this->claim($session);

        self::assertSame(['linked' => [$cloud], 'conflicts' => [], 'devices' => []], $answer);
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
        // The server gives the row its person at the end of the push that brings her device's (SEG-030); the claim finds it done.
        self::assertSame(self::LAURA, $this->memberOf($cloud, 'family@example.com'));

        self::assertSame(['linked' => [], 'conflicts' => [], 'devices' => []], $this->claim($this->family));

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

        self::assertSame(['linked' => [], 'conflicts' => [$cloud], 'devices' => []], $this->claim($secondSession));

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

        self::assertSame(['linked' => [], 'conflicts' => [], 'devices' => []], $this->claim($session));

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

    // ------------------------------------------------------ the new phone (7.5.3) ---

    /** A lost phone: an administrator took its person away, with a fence. The cloud does not give it back. */
    public function testAPhoneWhosePersonAnAdministratorTookAwayIsNotLinkedAgainByTheCloud(): void
    {
        [$cloud, $old, $oldSession] = $this->hugoWithAnAccount();
        $this->laptopSigns($cloud, 'device.assign', [['entity' => 'device', 'id' => $old->id, 'action' => 'update', 'fields' => ['member_id' => null, 'vouched_key' => null, 'trusted_through' => '']]]);
        self::assertNull($this->personOfDevice($old->id));

        $answer = $this->claim($oldSession);

        self::assertSame([], $answer['devices']);
        self::assertNull($this->personOfDevice($old->id), 'only an administrator gives it back');
    }

    public function testAPhoneWithARevocationIsNotLinkedAgainByTheCloud(): void
    {
        [$cloud, $old, $oldSession] = $this->hugoWithAnAccount();
        $this->laptopSigns($cloud, 'device.assign', [['entity' => 'device', 'id' => $old->id, 'action' => 'update', 'fields' => ['member_id' => null, 'vouched_key' => null]]]);
        $this->laptopSigns($cloud, 'device.remove', [['entity' => 'revocation', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['device_id' => $old->id, 'device_name' => 'Hugo']]]);

        self::assertSame([], $this->claim($oldSession)['devices']);
        self::assertNull($this->personOfDevice($old->id));
    }

    /** Jhonny, a reader with access, registered the id of Hugo's new phone with a key of his own before the phone did. */
    public function testAnIdJhonnyRegisteredFirstWithAKeyOfHisOwnIsNotLinkedToHugoWhenHugosPhoneClaims(): void
    {
        [$cloud] = $this->hugoWithAnAccount();
        $this->shareWithJhonny($cloud);
        $newId = Uuid::v4()->toRfc4122();
        $jhonnysCopy = $this->loginDevice($this->client, 'jhonny@example.com', self::PASSWORD, $newId, 'Phone');
        self::assertSame(1, $this->push($this->jhonny, $cloud, [SigningDevice::fromSession($jhonnysCopy)->registration(self::LOCAL, self::T + $this->tick())])['received']);
        $hugosNew = $this->loginDevice($this->client, 'hugo@example.com', self::PASSWORD, $newId, 'New phone');

        $answer = $this->claim($hugosNew);

        self::assertSame([], $answer['devices']);
        self::assertNull($this->personOfDevice($newId), 'the id stays nobody\'s');
        self::assertSame([], $this->cloudOperations($cloud, 'device.link_account'));
    }

    public function testAnAccountWithARowButNoPersonLinksNoDevice(): void
    {
        $cloud = $this->world();
        $this->shareWithJhonny($cloud, asTheReader: false);
        $mine = SigningDevice::fromSession($this->jhonny);
        // The family sends his registration: a row without a person gets nothing through the door (SEG-030).
        $this->push($this->family, $cloud, [$mine->registration(self::LOCAL, self::T + $this->tick())]);

        self::assertSame([], $this->claim($this->jhonny)['devices']);
        self::assertNull($this->personOfDevice($mine->id));
    }

    public function testTheNewPhoneOfAReaderIsLinkedAsAReaderAndWhatItUploadsIsARequest(): void
    {
        $cloud = $this->world();
        $oldSession = $this->signInNewDevice($this->client, 'reader2@example.com', self::PASSWORD, 'Reader');
        $old = SigningDevice::fromSession($oldSession);
        $this->push($this->family, $cloud, [$old->registration(self::LOCAL, self::T + $this->tick())]);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($old->id, self::READER, $old->publicKey)]);
        self::assertSame([$cloud], $this->claim($oldSession)['linked']);
        [$new, $newSession] = $this->newPhone($cloud, 'reader2@example.com', $oldSession);

        $answer = $this->claim($newSession);

        self::assertSame([$cloud], $answer['devices']);
        self::assertSame(self::READER, $this->personOfDevice($new->id));
        $before = $this->scalar("SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ? AND status = 'request'", [$cloud]);
        $this->push($newSession, $cloud, [$new->publisher(self::LOCAL, (int) (microtime(true) * 1000) + 60_000)]);
        self::assertSame($before + 1, $this->scalar("SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ? AND status = 'request'", [$cloud]), 'never more role than the person has');
    }

    public function testTheCloudWithoutThePowerLinksNoDeviceAndWritesNothing(): void
    {
        $cloud = $this->world(namesTheCloud: false);
        [$old, $oldSession] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($old->id, self::HUGO, $old->publicKey)]);
        $this->claim($oldSession);
        [$new, $newSession] = $this->newPhone($cloud, 'hugo@example.com', $oldSession);

        self::assertSame([], $this->claim($newSession)['devices']);

        self::assertNull($this->personOfDevice($new->id));
        self::assertSame([], $this->cloudOperations($cloud, null));
    }

    /** The account is already noted, then the administrator stops naming the cloud: it has no power there now. */
    public function testTheCloudWhosePowerWasTakenAwayLinksNoDevice(): void
    {
        [$cloud, , $oldSession] = $this->hugoWithAnAccount();
        // Dated after what the cloud has written (it uses the server's clock), as an administrator's phone would be.
        $this->laptopSigns($cloud, 'library.cloud', [['entity' => 'library', 'id' => self::LOCAL, 'action' => 'update', 'fields' => ['cloud_device_id' => '']]], (int) (microtime(true) * 1000) + 60_000);
        [$new, $newSession] = $this->newPhone($cloud, 'hugo@example.com', $oldSession);
        $operations = $this->scalar('SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ?', [$cloud]);

        self::assertSame([], $this->claim($newSession)['devices']);

        self::assertNull($this->personOfDevice($new->id));
        self::assertSame($operations, $this->scalar('SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ?', [$cloud]), 'nothing is written');
    }

    public function testADeviceThatAlreadyBelongsToAnotherPersonIsNotTouched(): void
    {
        [$cloud, , $oldSession] = $this->hugoWithAnAccount();
        [$new, $newSession] = $this->newPhone($cloud, 'hugo@example.com', $oldSession);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($new->id, self::READER, $new->publicKey)]);

        self::assertSame([], $this->claim($newSession)['devices']);

        self::assertSame(self::READER, $this->personOfDevice($new->id));
    }

    public function testTheNewPhoneOfAnEditorIsLinkedOnceAndHasItsRoleEverywhere(): void
    {
        [$cloud, , $oldSession] = $this->hugoWithAnAccount();
        [$new, $newSession] = $this->newPhone($cloud, 'hugo@example.com', $oldSession);
        self::assertNull($this->personOfDevice($new->id), 'registered, nobody\'s');

        // A body with ids is ignored: the same answer as without one.
        $answer = $this->call('POST', '/v1/cloud/libraries/claim', $newSession, ['library' => Uuid::v4()->toRfc4122(), 'member' => self::LAURA, 'device' => $this->laptop->id, 'devices' => [$this->laptop->id]]);
        self::assertSame(200, $answer['status']);
        self::assertSame(['linked' => [], 'conflicts' => [], 'devices' => [$cloud]], $answer['body']);

        self::assertSame(self::HUGO, $this->personOfDevice($new->id));
        self::assertSame($new->publicKey, $this->vouchedKeyOfDevice($new->id));
        $rows = $this->cloudOperations($cloud, 'device.link_account');
        self::assertCount(1, $rows);
        self::assertSame('applied', $rows[0]['status']);
        // It is an editor's device now: what it adds is applied.
        $before = $this->publishers($cloud);
        // Dated after the cloud's link, as the clock of a phone that has pulled it would be (the server writes with its own clock).
        $this->push($newSession, $cloud, [$new->publisher(self::LOCAL, (int) (microtime(true) * 1000) + 60_000, 'By the new phone')]);
        self::assertSame($before + 1, $this->publishers($cloud));
        // Twice: nothing more.
        self::assertSame(['linked' => [], 'conflicts' => [], 'devices' => []], $this->claim($newSession));
        self::assertCount(1, $this->cloudOperations($cloud, 'device.link_account'));
    }

    /**
     * Hugo's first phone, assigned and vouched, and his account claimed: linked, with the note of his account.
     *
     * @return array{string, SigningDevice, array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string}}
     */
    private function hugoWithAnAccount(): array
    {
        $cloud = $this->world();
        [$old, $session] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($old->id, self::HUGO, $old->publicKey)]);
        self::assertSame([$cloud], $this->claim($session)['linked']);

        return [$cloud, $old, $session];
    }

    /**
     * A new phone of the account: it signs in and registers itself in the library, through the access it has.
     *
     * @param array{headers: array<string, string>} $withAccess
     *
     * @return array{SigningDevice, array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string}}
     */
    private function newPhone(string $cloud, string $email, array $withAccess): array
    {
        $session = $this->loginDevice($this->client, $email, self::PASSWORD, Uuid::v4()->toRfc4122(), 'New phone');
        $device = SigningDevice::fromSession($session);
        self::assertSame(1, $this->push($withAccess, $cloud, [$device->registration(self::LOCAL, self::T + $this->tick())])['received']);

        return [$device, $session];
    }

    private function personOfDevice(string $device): ?string
    {
        $member = self::getContainer()->get(Connection::class)->fetchOne('SELECT m.id FROM catalog_devices d JOIN catalog_members m ON m.key = d.member_key WHERE d.id = ?', [$device]);

        return \is_string($member) ? $member : null;
    }

    private function vouchedKeyOfDevice(string $device): ?string
    {
        $key = self::getContainer()->get(Connection::class)->fetchOne('SELECT vouched_key FROM catalog_devices WHERE id = ?', [$device]);

        return \is_string($key) ? $key : null;
    }

    private function publishers(string $cloud): int
    {
        return $this->scalar('SELECT COUNT(*) FROM catalog_publishers WHERE cloud_library_id = ?', [$cloud]);
    }

    // ------------------------------------------------------ the cloud cuts the access (7.5.4b) ---

    /** A withdrawn device: the cloud stops giving it the library, with the proofs, and the account keeps its other devices. */
    public function testAWithdrawnDeviceGetsA403WithTheProofAndNothingElseWhileTheOtherDeviceOfTheAccountKeepsGoingIn(): void
    {
        [$cloud, $one, $oneSession, $two, $twoSession] = $this->jhonnyWithTwoDevices();
        $revocation = Uuid::v7()->toRfc4122();
        $this->laptopSigns($cloud, 'library.revoke', [['entity' => 'revocation', 'id' => $revocation, 'action' => 'create', 'fields' => ['device_id' => $one->id, 'device_name' => 'Jhonny']]]);

        $pull = $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $oneSession, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()]);
        $push = $this->push($oneSession, $cloud, [$one->publisher(self::LOCAL, (int) (microtime(true) * 1000) + 60_000)]);

        self::assertSame(403, $pull['status']);
        self::assertSame(['error', 'proofs'], array_keys($pull['body']), 'no operation of the library goes with it');
        $proofs = $pull['body']['proofs'] ?? null;
        \assert(\is_array($proofs) && \is_array($proofs[0] ?? null) && \is_array($proofs[0]['operation'] ?? null));
        // On a pull, with the vector of a device that has nothing: her earlier operations, up to the revocation (last).
        self::assertGreaterThan(1, \count($proofs));
        \assert(\is_array($proofs[\count($proofs) - 1]) && \is_array($proofs[\count($proofs) - 1]['operation'] ?? null));
        self::assertSame('library.revoke', $proofs[\count($proofs) - 1]['operation']['intent']);
        self::assertSame('access_withdrawn', $push['error'], 'the same for a push');
        self::assertSame(200, $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $twoSession, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()])['status'], 'the other device of the account');
        self::assertSame(0, $this->publishers($cloud), 'nothing the withdrawn device sent was stored');
    }

    /** The person is withdrawn: every device of the account, and a new one, is cut. */
    public function testAWithdrawnPersonLosesEveryDeviceOfTheAccountAndANewOneAndIsNotListedNorLinkedNorSubscribed(): void
    {
        [$cloud, $one, $oneSession, $two, $twoSession] = $this->jhonnyWithTwoDevices();
        $this->withdrawJhonny($cloud, $one, $two);
        $three = $this->loginDevice($this->client, 'jhonny@example.com', self::PASSWORD, Uuid::v4()->toRfc4122(), 'Third');

        foreach ([$oneSession, $twoSession, $three] as $session) {
            $pull = $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $session, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()]);
            self::assertSame(403, $pull['status']);
            self::assertSame('access_withdrawn', $pull['body']['error']);
        }
        $list = $this->call('GET', '/v1/cloud/libraries', $three)['body'];
        self::assertSame(['libraries' => [], 'withdrawn' => [$cloud]], $list);
        self::assertSame(['linked' => [], 'conflicts' => [], 'devices' => []], $this->claim($three));
        self::assertNull($this->memberOf($cloud, 'jhonny@example.com'), 'the row names nobody');
        $topics = $this->call('GET', '/v1/cloud/events', $three)['body']['topics'] ?? [];
        \assert(\is_array($topics));
        self::assertNotContains('urn:kosmotheka:cloud-library:'.$cloud, $topics);
        self::assertSame(1, $this->scalar('SELECT COUNT(*) FROM membership WHERE cloud_library_id = ? AND withdrawn_at IS NOT NULL', [$cloud]));
    }

    public function testTheCloudEmptiesTheAccountOfAWithdrawnPersonSignedAndAppliedAndOnlyOnce(): void
    {
        [$cloud, $one, , $two] = $this->jhonnyWithTwoDevices();
        self::assertNotNull($this->accountIdOf(self::READER), 'it was noted when he was linked');

        $this->withdrawJhonny($cloud, $one, $two);

        self::assertNull($this->accountIdOf(self::READER));
        $rows = $this->cloudOperations($cloud, 'member.unlink_account');
        self::assertCount(1, $rows);
        self::assertSame('applied', $rows[0]['status']);
        $body = json_decode($rows[0]['body'], false, flags: \JSON_THROW_ON_ERROR);
        \assert($body instanceof \stdClass);
        new OperationSignature()->verify($body, $this->cloudKey($cloud));
        $this->call('GET', '/v1/cloud/libraries', $this->jhonny);
        $this->call('GET', '/v1/cloud/libraries', $this->jhonny);
        self::assertCount(1, $this->cloudOperations($cloud, 'member.unlink_account'), 'the later requests write nothing');
    }

    /**
     * A cut device whose vector is behind: it gets the withdrawing author's earlier operations, up to the withdrawal,
     * so it can chain it; never what is dated after, never another author's. The vector decides how many, not what.
     */
    public function testACutDeviceBehindTheAdministratorGetsHerEarlierOperationsUpToTheWithdrawalAndNothingElse(): void
    {
        [$cloud, $one, $oneSession, $two] = $this->jhonnyWithTwoDevices();
        $earlier = $this->laptop->publisher(self::LOCAL, (int) (microtime(true) * 1000) + 10_000, 'Before');
        $this->push($this->family, $cloud, [$earlier]);
        $this->withdrawJhonny($cloud, $one, $two);
        $this->push($this->family, $cloud, [$this->laptop->publisher(self::LOCAL, (int) (microtime(true) * 1000) + 90_000, 'After')]);
        $connection = self::getContainer()->get(Connection::class);
        $withdrawal = $connection->fetchAssociative("SELECT operation_id, hlc FROM cloud_operation WHERE cloud_library_id = ? AND body::text LIKE '%member.withdraw%'", [$cloud]);
        \assert(\is_array($withdrawal) && \is_string($withdrawal['operation_id']) && \is_string($withdrawal['hlc']));
        $withdrawalId = $withdrawal['operation_id'];
        $withdrawalHlc = $withdrawal['hlc'];
        /** @var array<string, string> $authored */
        $authored = [];
        foreach ($connection->fetchAllAssociative('SELECT operation_id, hlc FROM cloud_operation WHERE cloud_library_id = ? AND author_device_id = ?', [$cloud, $this->laptop->id]) as $row) {
            \assert(\is_string($row['operation_id']) && \is_string($row['hlc']));
            $authored[$row['operation_id']] = $row['hlc'];
        }
        $earlierHlc = $earlier['hlc'];
        \assert(\is_string($earlierHlc));

        $vectors = [
            'empty' => [],
            'up to the earlier one' => [$this->laptop->id => $earlierHlc],
            'made up, of another author' => [$one->id => '0001727222222222-00000000-'.$one->id],
        ];
        foreach ($vectors as $name => $vector) {
            $pull = $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $oneSession, ['schemaVersion' => $this->schema(), 'vector' => $vector ?: new \stdClass()]);
            self::assertSame(403, $pull['status'], $name);
            $proofs = $pull['body']['proofs'] ?? null;
            \assert(\is_array($proofs));
            $ids = array_map(static fn (mixed $proof): string => \is_array($proof) && \is_array($proof['operation'] ?? null) && \is_string($proof['operation']['id'] ?? null) ? $proof['operation']['id'] : '', $proofs);
            $since = $vector[$this->laptop->id] ?? '';
            $expected = array_keys(array_filter($authored, static fn (string $hlc): bool => $hlc > $since && $hlc <= $withdrawalHlc));
            sort($expected);
            $got = $ids;
            sort($got);
            self::assertSame($expected, $got, "$name: her operations after the vector and up to the withdrawal, nothing else");
            self::assertContains($withdrawalId, $ids, $name);
            $hlcs = array_map(static fn (string $id): string => $authored[$id] ?? '', $ids);
            $sorted = $hlcs;
            sort($sorted);
            self::assertSame($sorted, $hlcs, "$name: in HLC order");
        }
    }

    public function testWithoutThePowerTheCloudWritesNothingAndTheCutHoldsAll(): void
    {
        $cloud = $this->world(namesTheCloud: false);
        [$one, $oneSession] = $this->jhonnyDevice($cloud, $this->jhonny);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($one->id, self::READER, $one->publicKey)]);
        self::assertSame([$cloud], $this->claim($oneSession)['linked']);

        $this->withdrawJhonny($cloud, $one);

        self::assertSame(403, $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $oneSession, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()])['status']);
        self::assertSame([], $this->cloudOperations($cloud, null));
    }

    /** Taking the mark off is not coming back: the account is linked again the usual way, or not at all. */
    public function testTakingTheMarkOffGivesNothingBackAndLinkingAgainDoes(): void
    {
        [$cloud, $one, $oneSession, $two, $twoSession] = $this->jhonnyWithTwoDevices();
        $this->withdrawJhonny($cloud, $one, $two);
        $back = (int) (microtime(true) * 1000) + 200_000;
        $this->laptopSigns($cloud, 'member.restore', [['entity' => 'member', 'id' => self::READER, 'action' => 'update', 'fields' => ['access_withdrawn' => false]]], $back);

        self::assertSame(403, $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $oneSession, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()])['status'], 'the mark alone');
        self::assertSame(['linked' => [], 'conflicts' => [], 'devices' => []], $this->claim($oneSession));

        // The administrator links a device again, vouching for its key; the account is linked the usual way.
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($two->id, self::READER, $two->publicKey)], $back + 1);
        $this->laptopSigns($cloud, 'library.grant', [['entity' => 'revocation', 'id' => $this->revocationOf($two->id), 'action' => 'delete', 'fields' => new \stdClass()]], $back + 2);
        $answer = $this->claim($twoSession);

        self::assertSame([$cloud], $answer['linked']);
        self::assertSame(self::READER, $this->memberOf($cloud, 'jhonny@example.com'));
        self::assertSame(200, $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $twoSession, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()])['status']);
    }

    public function testAWithdrawnPersonWhoTriesToTakeTheMarkOffOrDeleteHisRevocationIsTurnedDownAtTheDoorAndNothingChanges(): void
    {
        [$cloud, $one, $oneSession, $two] = $this->jhonnyWithTwoDevices();
        $this->withdrawJhonny($cloud, $one, $two);
        $before = [$this->scalar('SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ?', [$cloud]), $this->accessWithdrawnOf(self::READER), $this->scalar('SELECT COUNT(*) FROM catalog_revocations WHERE cloud_library_id = ?', [$cloud])];
        $at = (int) (microtime(true) * 1000) + 120_000;

        $mark = $this->push($oneSession, $cloud, [$one->sign(self::LOCAL, $at, 'member.restore', [['entity' => 'member', 'id' => self::READER, 'action' => 'update', 'fields' => ['access_withdrawn' => false]]])]);
        $revocation = $this->push($oneSession, $cloud, [$one->sign(self::LOCAL, $at + 1, 'library.grant', [['entity' => 'revocation', 'id' => $this->revocationOf($one->id), 'action' => 'delete', 'fields' => new \stdClass()]])]);

        self::assertSame('access_withdrawn', $mark['error']);
        self::assertSame('access_withdrawn', $revocation['error']);
        self::assertSame($before, [$this->scalar('SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = ?', [$cloud]), $this->accessWithdrawnOf(self::READER), $this->scalar('SELECT COUNT(*) FROM catalog_revocations WHERE cloud_library_id = ?', [$cloud])]);
    }

    /** The id of a withdrawn device with another key is another device (of another account, with access): it is not cut for that. */
    public function testADeviceWithTheIdOfAWithdrawnDeviceAndAnotherKeyIsNotCut(): void
    {
        [$cloud, $one, $oneSession] = $this->jhonnyWithTwoDevices();
        [, , $hugoSession] = [null, null, $this->hugoJoins($cloud)];
        $this->laptopSigns($cloud, 'library.revoke', [['entity' => 'revocation', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['device_id' => $one->id, 'device_name' => 'Jhonny']]]);
        $sameId = $this->loginDevice($this->client, 'hugo@example.com', self::PASSWORD, $one->id, 'Same id, another key');

        self::assertSame(200, $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $sameId, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()])['status']);
        self::assertSame(403, $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $oneSession, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()])['status']);
        self::assertNotSame($hugoSession['token'], $sameId['token']);
    }

    /** SEG-031: the proofs are for the very device a revocation names (id and key), never for whoever signs in with its id. */
    public function testAnAccountThatWasCutGetsNoProofsOfAnotherDevicesLaterRevocationByUsingItsId(): void
    {
        [$cloud, $one, , $two] = $this->jhonnyWithTwoDevices();
        [$hugo] = $this->hugo($cloud);
        $this->withdrawJhonny($cloud, $one, $two);
        $later = (int) (microtime(true) * 1000) + 60_000;
        $this->laptopSigns($cloud, 'publisher.create', [['entity' => 'publisher', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['name' => 'After Jhonny left']]], $later);
        $this->laptopSigns($cloud, 'library.revoke', [['entity' => 'revocation', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['device_id' => $hugo->id, 'device_name' => 'Hugo']]], $later + 1_000);
        $hugosId = $this->loginDevice($this->client, 'jhonny@example.com', self::PASSWORD, $hugo->id, 'The id of Hugo, a key of his own');

        $pull = $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $hugosId, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()]);

        self::assertSame(403, $pull['status']);
        self::assertSame([], $pull['body']['proofs'] ?? null, 'nothing written after he was cut reaches him');
    }

    /** SEG-030: whoever uploaded is reached by a withdrawal too, without her app ever claiming her person. */
    public function testTheOwnerWhoNeverClaimedIsCutWhenHerPersonIsWithdrawnAlsoFromANewDevice(): void
    {
        $cloud = $this->world();
        self::assertSame(self::LAURA, $this->memberOf($cloud, 'family@example.com'), 'the server gave the row its person, unasked');
        $this->laptopSigns($cloud, 'member.withdraw', [
            ['entity' => 'member', 'id' => self::LAURA, 'action' => 'update', 'fields' => ['access_withdrawn' => true]],
            // No fence here: an empty mark would turn the laptop's link into a request, and the push would be undone (D93).
            ['entity' => 'device', 'id' => $this->laptop->id, 'action' => 'update', 'fields' => ['member_id' => null, 'vouched_key' => null]],
            ['entity' => 'revocation', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['device_id' => $this->laptop->id, 'device_name' => 'Laptop']],
        ]);
        $new = $this->loginDevice($this->client, 'family@example.com', self::PASSWORD, Uuid::v4()->toRfc4122(), 'A new device');

        $pull = $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $new, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()]);

        self::assertSame(403, $pull['status']);
        self::assertSame(['libraries' => [], 'withdrawn' => [$cloud]], $this->call('GET', '/v1/cloud/libraries', $new)['body']);
    }

    /** SEG-030: in a library with roles an account that is nobody there gets nothing from the cloud, and gets it back with a person. */
    public function testAnOwnerWhoIsNobodyInALibraryWithRolesGetsNothingUntilSheHasAPerson(): void
    {
        $cloud = $this->call('POST', '/v1/cloud/libraries', $this->family)['body']['id'] ?? null;
        \assert(\is_string($cloud));
        $other = SigningDevice::fromSession($this->signInNewDevice($this->client, 'other@example.com'));
        self::assertSame(5, $this->push($this->family, $cloud, [
            $this->laptop->registration(self::LOCAL, self::T + $this->tick()),
            $this->laptop->sign(self::LOCAL, self::T + $this->tick(), 'library.create', [['entity' => 'library', 'id' => self::LOCAL, 'action' => 'create', 'fields' => ['name' => 'Family']]]),
            $this->laptop->sign(self::LOCAL, self::T + $this->tick(), 'library.link', [['entity' => 'library', 'id' => self::LOCAL, 'action' => 'update', 'fields' => ['cloud_library_id' => $cloud, 'cloud_server' => SigningDevice::SERVER]]]),
            $other->registration(self::LOCAL, self::T + $this->tick()),
            // Somebody else turns the roles on and is the only person: the uploader's laptop is nobody's.
            $other->sign(self::LOCAL, self::T + $this->tick(), 'family.roles', [
                ['entity' => 'member', 'id' => self::LAURA, 'action' => 'create', 'fields' => ['name' => 'Laura', 'role' => 'admin']],
                $this->assign($other->id, self::LAURA, $other->publicKey),
            ]),
        ])['received']);

        $nobody = $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $this->family, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()]);
        self::assertSame(403, $nobody['status']);
        self::assertSame([], $nobody['body']['proofs'] ?? null);
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM membership WHERE cloud_library_id = ? AND withdrawn_at IS NOT NULL', [$cloud]), 'not a cut: she was never somebody');

        // She may still send, as a device of nobody; and once an administrator says whose her laptop is, she is in.
        self::assertSame(1, $this->push($this->family, $cloud, [$this->laptop->publisher(self::LOCAL, self::T + $this->tick())])['received']);
        self::assertSame(1, $this->push($this->family, $cloud, [$other->sign(self::LOCAL, self::T + $this->tick(), 'member.add', [['entity' => 'member', 'id' => self::HUGO, 'action' => 'create', 'fields' => ['name' => 'Hugo', 'role' => 'editor']]])])['received']);
        self::assertSame(1, $this->push($this->family, $cloud, [$other->sign(self::LOCAL, self::T + $this->tick(), 'device.assign', [$this->assign($this->laptop->id, self::HUGO, $this->laptop->publicKey)])])['received']);
        self::assertSame(200, $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $this->family, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()])['status']);
        self::assertSame(self::HUGO, $this->memberOf($cloud, 'family@example.com'));
    }

    /**
     * Hugo, an editor with his account linked, in a library that already has Jhonny.
     *
     * @return array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string}
     */
    private function hugoJoins(string $cloud): array
    {
        [$hugo, $session] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($hugo->id, self::HUGO, $hugo->publicKey)]);
        self::assertSame([$cloud], $this->claim($session)['linked']);

        return $session;
    }

    public function testAnAccountThatNeverHadTheLibraryGetsA404NotA403(): void
    {
        [$cloud, $one, , $two] = $this->jhonnyWithTwoDevices();
        $this->withdrawJhonny($cloud, $one, $two);
        $stranger = $this->signInNewDevice($this->client, 'mallory@example.com');

        $pull = $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $stranger, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()]);

        self::assertSame(404, $pull['status']);
        self::assertSame(['error' => 'not_found'], $pull['body']);
        self::assertSame(['libraries' => [], 'withdrawn' => []], $this->call('GET', '/v1/cloud/libraries', $stranger)['body']);
    }

    public function testWhatTheClientSendsDoesNotChangeTheCut(): void
    {
        [$cloud, $one, $oneSession, $two] = $this->jhonnyWithTwoDevices();
        $this->withdrawJhonny($cloud, $one, $two);

        $plain = $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $oneSession, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()]);
        $other = $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $oneSession, ['schemaVersion' => $this->schema(), 'vector' => [$one->id => '0001727222222222-00000000-'.$one->id]]);
        $link = $this->call('POST', "/v1/cloud/libraries/$cloud/link", $oneSession, ['proofs' => [], 'library' => Uuid::v4()->toRfc4122(), 'member' => self::LAURA]);

        self::assertSame([403, 403, 403], [$plain['status'], $other['status'], $link['status']]);
        self::assertSame($plain['body']['error'], $other['body']['error']);
        self::assertSame($plain['body']['error'], $link['body']['error']);
        // Without a vector (the link route) only the revocation goes; the vector decides how many earlier ones go with it, no more.
        self::assertIsArray($link['body']['proofs']);
        self::assertCount(1, $link['body']['proofs']);
        self::assertSame($plain['body']['proofs'], $other['body']['proofs'], 'a vector of another author changes nothing for hers');
    }

    /** The withdrawal reaches the cloud late, dated before something else already stored: the cut holds all the same. */
    public function testAWithdrawalThatArrivesLateStillCutsTheAccount(): void
    {
        [$cloud, $one, $oneSession, $two] = $this->jhonnyWithTwoDevices();
        [$hugo] = $this->hugo($cloud);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($hugo->id, self::HUGO, $hugo->publicKey)], (int) (microtime(true) * 1000) + 10_000);
        // Another author's later operation is stored first; the withdrawal, dated before it, arrives after.
        $late = (int) (microtime(true) * 1000) + 90_000;
        $withdrawal = $this->laptop->sign(self::LOCAL, $late - 10_000, 'member.withdraw', [
            ['entity' => 'member', 'id' => self::READER, 'action' => 'update', 'fields' => ['access_withdrawn' => true]],
            ['entity' => 'device', 'id' => $one->id, 'action' => 'update', 'fields' => ['member_id' => null, 'vouched_key' => null, 'trusted_through' => '']],
            ['entity' => 'device', 'id' => $two->id, 'action' => 'update', 'fields' => ['member_id' => null, 'vouched_key' => null, 'trusted_through' => '']],
        ]);
        self::assertSame(1, $this->push($this->family, $cloud, [$hugo->publisher(self::LOCAL, $late, 'Stored first')])['received']);
        self::assertSame(200, $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $oneSession, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()])['status']);

        self::assertSame(1, $this->push($this->family, $cloud, [$withdrawal])['received']);

        self::assertSame(403, $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $oneSession, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()])['status']);
        self::assertNull($this->accountIdOf(self::READER));
    }

    /**
     * Jhonny, a reader, with two devices of one account, both assigned and vouched, and the account linked.
     *
     * @return array{string, SigningDevice, array{headers: array<string, string>}, SigningDevice, array{headers: array<string, string>}}
     */
    private function jhonnyWithTwoDevices(): array
    {
        $cloud = $this->world();
        [$one, $oneSession] = $this->jhonnyDevice($cloud, $this->jhonny);
        $twoSession = $this->loginDevice($this->client, 'jhonny@example.com', self::PASSWORD, Uuid::v4()->toRfc4122(), 'Second');
        [$two] = $this->jhonnyDevice($cloud, $twoSession);
        $this->laptopSigns($cloud, 'device.assign', [$this->assign($one->id, self::READER, $one->publicKey), $this->assign($two->id, self::READER, $two->publicKey)]);
        self::assertSame([$cloud], $this->claim($oneSession)['linked']);

        return [$cloud, $one, $oneSession, $two, $twoSession];
    }

    /**
     * A device of Jhonny's account, registered in the library (pushed by the family: whoever pushes, the signature is its own).
     *
     * @param array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} $session
     *
     * @return array{SigningDevice, array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string}}
     */
    private function jhonnyDevice(string $cloud, array $session): array
    {
        $device = SigningDevice::fromSession($session);
        $this->push($this->family, $cloud, [$device->registration(self::LOCAL, self::T + $this->tick())]);

        return [$device, $session];
    }

    /** What the app writes in one operation (D98): the mark, and for each device no person, a fence and a revocation. */
    private function withdrawJhonny(string $cloud, SigningDevice ...$devices): void
    {
        $changes = [['entity' => 'member', 'id' => self::READER, 'action' => 'update', 'fields' => ['access_withdrawn' => true]]];
        foreach ($devices as $device) {
            $changes[] = ['entity' => 'device', 'id' => $device->id, 'action' => 'update', 'fields' => ['member_id' => null, 'vouched_key' => null, 'trusted_through' => '']];
            $changes[] = ['entity' => 'revocation', 'id' => Uuid::v7()->toRfc4122(), 'action' => 'create', 'fields' => ['device_id' => $device->id, 'device_name' => 'Jhonny']];
        }
        // Dated after what the cloud wrote when it linked him (the server writes with its own clock).
        $this->laptopSigns($cloud, 'member.withdraw', $changes, (int) (microtime(true) * 1000) + 30_000);
    }

    private function revocationOf(string $device): string
    {
        $id = self::getContainer()->get(Connection::class)->fetchOne('SELECT id FROM catalog_revocations WHERE device_id = ?', [$device]);
        \assert(\is_string($id));

        return $id;
    }

    private function accessWithdrawnOf(string $member): ?bool
    {
        $value = self::getContainer()->get(Connection::class)->fetchOne('SELECT access_withdrawn FROM catalog_members WHERE id = ?', [$member]);

        return \is_bool($value) ? $value : null;
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
    private function laptopSigns(string $cloud, string $intent, array $changes, ?int $at = null): void
    {
        $response = $this->push($this->family, $cloud, [$this->laptop->sign(self::LOCAL, $at ?? self::T + $this->tick(), $intent, $changes)]);
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
     * @return array{linked: list<string>, conflicts: list<string>, devices: list<string>}
     */
    private function claim(array $session): array
    {
        $answer = $this->call('POST', '/v1/cloud/libraries/claim', $session);
        self::assertSame(200, $answer['status']);

        /** @var array{linked: list<string>, conflicts: list<string>, devices: list<string>} $body */
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

    /**
     * Jhonny's account gets a row in the library, as the reader of `world()`: in a library with roles an account
     * reaches the cloud as somebody (SEG-030). Without a person the row gives him nothing.
     */
    private function shareWithJhonny(string $cloud, bool $asTheReader = true): void
    {
        $em = self::getContainer()->get(EntityManagerInterface::class);
        $user = $em->getRepository(User::class)->findOneBy(['email' => 'jhonny@example.com']) ?? self::fail('No Jhonny.');
        $library = $em->find(CloudLibrary::class, Uuid::fromString($cloud)) ?? self::fail('No library.');
        $em->persist(new Membership($user, $library, $asTheReader ? Uuid::fromString(self::READER) : null));
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
        foreach ($this->libraryList($this->call('GET', '/v1/cloud/libraries', $this->family)['body']) as $library) {
            if (\is_array($library) && ($library['id'] ?? null) === $cloud && \is_string($library['cloudDeviceId'] ?? null)) {
                return $library['cloudDeviceId'];
            }
        }

        self::fail('Not listed.');
    }

    private function cloudKey(string $cloud): string
    {
        foreach ($this->libraryList($this->call('GET', '/v1/cloud/libraries', $this->family)['body']) as $library) {
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

    /**
     * The `libraries` of a listing answer.
     *
     * @param array<mixed> $body
     *
     * @return array<mixed>
     */
    private function libraryList(array $body): array
    {
        $libraries = $body['libraries'] ?? null;
        \assert(\is_array($libraries));

        return $libraries;
    }
}
