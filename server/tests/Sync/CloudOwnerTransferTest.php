<?php

declare(strict_types=1);

namespace App\Tests\Sync;

use App\DataFixtures\Cast;
use App\Entity\Catalog\Member;
use App\Entity\CloudLibrary;
use App\Entity\Membership;
use App\Entity\User;
use App\Sync\CloudLibraryCreator;
use App\Tests\Support\CatalogState;
use App\Tests\Support\DeviceAuthTestTrait;
use App\Tests\Support\Spec;
use Doctrine\DBAL\Connection;
use Doctrine\DBAL\Exception\DriverException;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Console\Application;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\Console\Tester\CommandTester;
use Symfony\Component\Uid\Uuid;

/**
 * D99: a cloud library is not lost with the account of whoever uploaded it. Deleting the account hands each library to
 * another account that has an active place in it, an administrator's first; a library nobody else can reach goes with
 * the account. Ana uploads, Bea administers, Carlos reads.
 */
final class CloudOwnerTransferTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string PASSWORD = 'correct horse battery';

    private KernelBrowser $client;
    /** @var array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private array $ana;

    protected function setUp(): void
    {
        $this->client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();
        $this->ana = $this->signInNewDevice($this->client, 'ana@example.com');
    }

    // --- 1. The library stays, with a new owner --------------------------------------------------------------------

    public function testTheLibraryStaysWithTheAdministratorAndTheOthersKeepWorkingWithIt(): void
    {
        $library = $this->libraryOf('ana@example.com');
        $bea = $this->place($library, 'bea@example.com', 'admin');
        $carlos = $this->place($library, 'carlos@example.com', 'reader');

        $this->deleteAna();

        self::assertSame(204, $this->client->getResponse()->getStatusCode());
        self::assertSame($this->idOf('bea@example.com'), $this->ownerOf($library));
        self::assertSame(1, $this->rowsIn('cloud_library'), 'The library is still in the cloud.');
        self::assertSame(2, $this->rowsIn('membership'), 'Bea and Carlos keep their place.');
        foreach ([$bea, $carlos] as $session) {
            self::assertSame([$library->getId()->toRfc4122()], array_column($this->listed($session), 'id'));
            self::assertSame(200, $this->pull($session, $library)['status']);
        }
    }

    public function testTheChangeOfOwnerIsWrittenInTheActivityLogByIdAndNeverByEmail(): void
    {
        $library = $this->libraryOf('ana@example.com');
        $this->place($library, 'bea@example.com', 'admin');
        $anaId = $this->idOf('ana@example.com');

        $this->deleteAna();

        $rows = self::getContainer()->get(Connection::class)->fetchAllAssociative("SELECT * FROM activity_log WHERE kind = 'library_owner_changed'");
        self::assertCount(1, $rows);
        self::assertSame($anaId, $rows[0]['account_id']);
        self::assertSame($this->idOf('bea@example.com'), $rows[0]['subject_id']);
        self::assertSame($library->getId()->toRfc4122(), $rows[0]['library_id']);
        self::assertStringNotContainsString('example.com', json_encode($rows, \JSON_THROW_ON_ERROR));
    }

    public function testOfSeveralAdministratorsTheOneWithTheOldestPlaceIsTheOwner(): void
    {
        $library = $this->libraryOf('ana@example.com');
        $this->place($library, 'late-admin@example.com', 'admin', 3);
        $this->place($library, 'early-admin@example.com', 'admin', 1);
        $this->place($library, 'middle-admin@example.com', 'admin', 2);

        $this->deleteAna();

        self::assertSame($this->idOf('early-admin@example.com'), $this->ownerOf($library));
    }

    // --- 2. No administrator with an account: anyone else --------------------------------------------------------

    public function testWithNoOtherAdministratorTheOldestOtherPersonIsTheOwner(): void
    {
        $library = $this->libraryOf('ana@example.com');
        $this->place($library, 'dani@example.com', 'reader', 2);
        $this->place($library, 'carlos@example.com', 'editor', 1);

        $this->deleteAna();

        self::assertSame($this->idOf('carlos@example.com'), $this->ownerOf($library));
        self::assertSame(1, $this->rowsIn('cloud_library'));
    }

    // --- 3. A person without access never receives it -----------------------------------------------------------

    public function testAPersonWhoseAccessWasWithdrawnOrWhoseMembershipWasNeverReceivesTheLibrary(): void
    {
        $library = $this->libraryOf('ana@example.com');
        $this->place($library, 'cut-admin@example.com', 'admin', 1, accessWithdrawn: true);
        $this->place($library, 'withdrawn-admin@example.com', 'admin', 2, membershipWithdrawn: true);
        $this->place($library, 'dani@example.com', 'reader', 3);

        $this->deleteAna();

        self::assertSame($this->idOf('dani@example.com'), $this->ownerOf($library), 'Only the reader with a place gets it.');
    }

    public function testIfOnlyPeopleWithoutAccessAreLeftTheLibraryGoesWithTheAccount(): void
    {
        $library = $this->libraryOf('ana@example.com');
        $this->place($library, 'cut-admin@example.com', 'admin', 1, accessWithdrawn: true);
        $this->place($library, 'withdrawn@example.com', 'reader', 2, membershipWithdrawn: true);
        $id = $library->getId()->toRfc4122();

        $this->deleteAna();

        self::assertSame(204, $this->client->getResponse()->getStatusCode());
        self::assertSame(0, $this->countWhere('cloud_library', 'id', $id));
        self::assertSame(0, $this->countWhere('membership', 'cloud_library_id', $id));
    }

    // --- 4. Alone ----------------------------------------------------------------------------------------------

    public function testAnaAloneInHerLibraryTakesItWithHerAndLeavesNoRows(): void
    {
        $library = $this->libraryOf('ana@example.com');
        $id = $library->getId()->toRfc4122();

        $this->deleteAna();

        self::assertSame(204, $this->client->getResponse()->getStatusCode());
        foreach (['cloud_library' => 'id', 'membership' => 'cloud_library_id', 'cloud_operation' => 'cloud_library_id', 'cloud_device_key' => 'cloud_library_id', 'catalog_members' => 'cloud_library_id'] as $table => $column) {
            self::assertSame(0, $this->countWhere($table, $column, $id), $table);
        }
    }

    public function testAnotherLibraryAnaDoesNotOwnIsNotTouched(): void
    {
        $bea = $this->signInNewDevice($this->client, 'bea@example.com');
        $theirs = $this->libraryOf('bea@example.com');
        $this->place($theirs, 'ana@example.com', 'admin');

        $this->deleteAna();

        self::assertSame($this->idOf('bea@example.com'), $this->ownerOf($theirs));
        self::assertSame(1, $this->rowsIn('cloud_library'));
        self::assertSame(1, $this->rowsIn('membership'), 'Only Ana\'s place in Bea\'s library is gone.');
        self::assertNotSame([], $bea['headers']);
    }

    // --- 5. The database does not do it by itself ------------------------------------------------------------------

    public function testDeletingTheUserRowOfAnOwnerDirectlyFailsAndTheLibraryStays(): void
    {
        $this->libraryOf('ana@example.com');
        $connection = self::getContainer()->get(Connection::class);

        // A savepoint of its own: the failed statement aborts the transaction it runs in.
        $connection->beginTransaction();
        try {
            $connection->executeStatement('DELETE FROM "user" WHERE id = ?', [$this->idOf('ana@example.com')]);
            $connection->commit();
            self::fail('The database deleted the owner and, with her, the library.');
        } catch (DriverException $exception) {
            $connection->rollBack();
            // 23001: restrict_violation.
            self::assertSame('23001', $exception->getSQLState());
        }
        self::assertSame(1, $this->rowsIn('cloud_library'));
    }

    // --- 6. The library after the change is the same -----------------------------------------------------------------

    public function testRebuildingAfterTheChangeOfOwnerGivesTheCatalogItGaveBefore(): void
    {
        $tester = new CommandTester(new Application($this->client->getKernel())->find('doctrine:fixtures:load'));
        $tester->execute([], ['interactive' => false]);
        $tester->assertCommandIsSuccessful();
        $this->em()->clear();

        $user = $this->em()->getRepository(User::class)->findOneBy(['email' => Cast::ANA]) ?? self::fail('No Ana.');
        $garcia = $this->em()->getRepository(CloudLibrary::class)->findOneBy(['owner' => $user]) ?? self::fail('Ana owns no library.');
        $id = $garcia->getId()->toRfc4122();
        $this->rebuild($id);
        $before = $this->stateOf($id);
        $statuses = $this->statusesOf($id);
        self::assertNotSame([], $before['rows']);

        // Ana deletes her account through the route, with the laptop the fixtures signed in; Hugo, a reader with an account, is the only other place.
        $hugo = $this->em()->getRepository(User::class)->findOneBy(['email' => Cast::HUGO]) ?? self::fail('No Hugo.');
        $laptop = Cast::anaLaptop();
        $this->client->request('DELETE', '/v1/account', server: ['HTTP_X-Device-Id' => $laptop->id, 'HTTP_Authorization' => 'Bearer '.Cast::token($laptop), 'CONTENT_TYPE' => 'application/json'], content: json_encode(['password' => Cast::PASSWORD], \JSON_THROW_ON_ERROR));
        self::assertSame(204, $this->client->getResponse()->getStatusCode());
        $this->em()->clear();

        self::assertSame($hugo->getId()->toRfc4122(), $this->ownerOf($this->em()->find(CloudLibrary::class, Uuid::fromString($id)) ?? self::fail('Gone.')));
        $this->rebuild($id);
        self::assertEquals($before, $this->stateOf($id));
        self::assertSame($statuses, $this->statusesOf($id));
    }

    // --- Helpers -----------------------------------------------------------------------------------------------------

    /** The container is new after every request of the client, and so is its entity manager. */
    private function em(): EntityManagerInterface
    {
        return self::getContainer()->get(EntityManagerInterface::class);
    }

    private function deleteAna(): void
    {
        $this->client->request('DELETE', '/v1/account', server: [...$this->ana['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode(['password' => self::PASSWORD], \JSON_THROW_ON_ERROR));
        $this->em()->clear();
    }

    private function libraryOf(string $email): CloudLibrary
    {
        $owner = $this->em()->getRepository(User::class)->findOneBy(['email' => $email]) ?? self::fail('No '.$email);

        return self::getContainer()->get(CloudLibraryCreator::class)->create($owner);
    }

    /**
     * A person of the library with an account of their own: the catalog row (role, access) and the membership that says which account is it.
     *
     * @return array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string}
     */
    private function place(CloudLibrary $library, string $email, string $role, int $minutesAgo = 0, bool $accessWithdrawn = false, bool $membershipWithdrawn = false): array
    {
        $known = $this->em()->getRepository(User::class)->findOneBy(['email' => $email]);
        $session = $known instanceof User ? $this->ana : $this->signInNewDevice($this->client, $email);
        $user = $this->em()->getRepository(User::class)->findOneBy(['email' => $email]) ?? self::fail('No '.$email);
        $library = $this->em()->find(CloudLibrary::class, $library->getId()) ?? self::fail('No library.');
        $person = Uuid::v7();

        $member = new Member($library, $person->toRfc4122());
        $member->setName(ucfirst(explode('@', $email)[0]));
        $member->setRole($role);
        $member->setAccessWithdrawn($accessWithdrawn ? true : null);
        $this->em()->persist($member);

        $membership = new Membership($user, $library, $person);
        new \ReflectionProperty($membership, 'createdAt')->setValue($membership, new \DateTimeImmutable(\sprintf('-%d minutes', 100 - $minutesAgo)));
        if ($membershipWithdrawn) {
            $membership->withdraw(new \DateTimeImmutable());
        }
        $this->em()->persist($membership);
        $this->em()->flush();

        return $session;
    }

    private function idOf(string $email): string
    {
        $id = self::getContainer()->get(Connection::class)->fetchOne('SELECT id FROM "user" WHERE email = ?', [$email]);
        \assert(\is_string($id));

        return $id;
    }

    private function ownerOf(CloudLibrary $library): string
    {
        $owner = self::getContainer()->get(Connection::class)->fetchOne('SELECT owner_id FROM cloud_library WHERE id = ?', [$library->getId()->toRfc4122()]);
        \assert(\is_string($owner));

        return $owner;
    }

    private function rowsIn(string $table): int
    {
        return $this->asInt(self::getContainer()->get(Connection::class)->fetchOne("SELECT COUNT(*) FROM $table"));
    }

    private function asInt(mixed $count): int
    {
        \assert(\is_int($count) || \is_string($count));

        return (int) $count;
    }

    private function countWhere(string $table, string $column, string $value): int
    {
        return $this->asInt(self::getContainer()->get(Connection::class)->fetchOne("SELECT COUNT(*) FROM $table WHERE $column = ?", [$value]));
    }

    /**
     * @param array{headers: array<string, string>} $session
     *
     * @return list<array{id: string}>
     */
    private function listed(array $session): array
    {
        $this->client->request('GET', '/v1/cloud/libraries', server: $session['headers']);
        /** @var array{libraries: list<array{id: string}>} $body */
        $body = json_decode((string) $this->client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);

        return $body['libraries'];
    }

    /**
     * @param array{headers: array<string, string>} $session
     *
     * @return array{status: int}
     */
    private function pull(array $session, CloudLibrary $library): array
    {
        $version = Spec::json('sync.json')['schemaVersion'] ?? null;
        $this->client->request('POST', '/v1/cloud/libraries/'.$library->getId()->toRfc4122().'/pull', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode(['schemaVersion' => $version, 'vector' => new \stdClass()], \JSON_THROW_ON_ERROR));

        return ['status' => $this->client->getResponse()->getStatusCode()];
    }

    private function rebuild(string $id): void
    {
        $tester = new CommandTester(new Application($this->client->getKernel())->find('app:catalog:rebuild'));
        $tester->execute(['cloud-library' => $id]);
        $tester->assertCommandIsSuccessful();
        $this->em()->clear();
    }

    /** @return array{rows: array<string, array<string, mixed>>, versions: array<string, string>, tombstones: array<string, string>} */
    private function stateOf(string $id): array
    {
        $state = CatalogState::of($this->em(), $this->em()->find(CloudLibrary::class, Uuid::fromString($id)) ?? self::fail('No library.'));
        $this->em()->clear();

        return $state;
    }

    /** @return array<string, string> */
    private function statusesOf(string $id): array
    {
        /** @var array<string, string> $statuses */
        $statuses = self::getContainer()->get(Connection::class)->fetchAllKeyValue('SELECT operation_id, COALESCE(status, \'none\') FROM cloud_operation WHERE cloud_library_id = ? ORDER BY operation_id', [$id]);

        return $statuses;
    }
}
