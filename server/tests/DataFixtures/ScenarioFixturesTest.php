<?php

declare(strict_types=1);

namespace App\Tests\DataFixtures;

use App\DataFixtures\Cast;
use App\DataFixtures\SigningDevice;
use App\Tests\Support\Spec;
use Doctrine\DBAL\Connection;
use Symfony\Bundle\FrameworkBundle\Console\Application;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\Console\Tester\CommandTester;
use Symfony\Component\HttpFoundation\Response;

/**
 * The scenarios of docs/development/fixtures.md, loaded into the test database
 * the way `make fixtures` loads them into the dev one. The load itself already
 * proves that every refused attempt was refused (it stops otherwise); these
 * tests assert the facts a reader of the database is meant to see.
 */
final class ScenarioFixturesTest extends WebTestCase
{
    private KernelBrowser $client;
    private Connection $connection;

    protected function setUp(): void
    {
        $this->client = self::createClient();
        // The rate limiter cache is not rolled back with the database.
        self::getContainer()->get('cache.rate_limiter')->clear();
        $this->connection = self::getContainer()->get(Connection::class);

        $this->load();
    }

    public function testTheCastHasItsAccountsAndDevices(): void
    {
        self::assertSame(4, $this->rowsIn('"user"'));
        self::assertSame(
            ['ana@garcia.test' => 2, 'hugo@garcia.test' => 1, 'jhonny@example.test' => 2, 'kenji@tanaka.test' => 1],
            $this->devicesPerAccount(),
        );
    }

    public function testTheCastIsWrittenForTheApp(): void
    {
        $file = self::getContainer()->getParameter('app.fixtures_cast_file');
        self::assertFileExists($file);
        /** @var array{devices: list<array{folder: string, email: string, deviceId: string, signingSeed: string, token: string, server: string, user: array{id: string}}>} $cast */
        $cast = json_decode((string) file_get_contents($file), true, flags: \JSON_THROW_ON_ERROR);

        self::assertSame(array_keys(Cast::devices()), array_column($cast['devices'], 'folder'));
        foreach ($cast['devices'] as $entry) {
            $expected = Cast::devices()[$entry['folder']];
            $device = $expected['device'];
            self::assertSame($expected['email'], $entry['email']);
            self::assertSame($device->id, $entry['deviceId']);
            self::assertSame($device->seed, $entry['signingSeed']);
            self::assertSame(64, \strlen($entry['signingSeed']));
            self::assertSame(Cast::token($device), $entry['token']);
            self::assertSame('http://localhost:8095', $entry['server']);
            self::assertSame(
                $this->connection->fetchOne('SELECT id FROM "user" WHERE email = :email', ['email' => $expected['email']]),
                $entry['user']['id'],
            );
        }
    }

    public function testAnaUploadsBibliotecaGarciaWithItsHistory(): void
    {
        $garcia = $this->cloudLibrariesOf(Cast::GARCIA_LOCAL);

        self::assertCount(2, $garcia, 'The Garcías\' own copy and the one Jhonny made of its ids.');
        $ana = $this->idOfLibraryOwnedBy(Cast::ANA);
        self::assertContains($ana, $garcia);
        // 13 of the upload, 1 of the phone, the laptop's older one, the cloud's own registration (D95) and its note of Hugo's account (7.5.2b-2).
        self::assertSame(17, $this->operationsIn($ana));
        // Three authors registered their keys: the laptop, the phone and Hugo's phone; and the cloud's device, since the library was created (D95).
        self::assertSame(4, $this->rowsIn('cloud_device_key WHERE cloud_library_id = '.$this->quote($ana)));
        // Hugo's mark and request, and Ana's roles, are in the history as operations.
        self::assertSame(1, $this->operationsWithChange($ana, 'mark'));
        self::assertSame(1, $this->operationsWithChange($ana, 'change_request'));
        self::assertSame(4, $this->operationsWithChange($ana, 'member'));
        self::assertSame(1, $this->operationsWithChange($ana, 'member', ['role' => 'admin']));
        self::assertSame(1, $this->operationsWithChange($ana, 'member', ['role' => 'reader']));
    }

    public function testLuciaIsOnlyAMemberRowOfTheOperations(): void
    {
        $ana = $this->idOfLibraryOwnedBy(Cast::ANA);
        $lucia = Cast::id('member:lucia');

        self::assertSame(1, $this->operationsWithChange($ana, 'member', ['name' => 'Lucía']));
        self::assertSame(1, $this->operationsWithChange($ana, 'copy', ['owner_id' => $lucia]));
        self::assertSame(0, $this->operationsWithChange($ana, 'device', ['member_id' => $lucia]));
        self::assertSame(0, $this->rowsIn('"user" WHERE name LIKE \'Luc%\''));
        self::assertSame(0, $this->rowsIn('device WHERE name LIKE \'Luc%\''));
    }

    public function testHugoClaimedHisAccessThroughHisVouchedPhone(): void
    {
        // Ana's, Kenji's and Jhonny's two owner rows, and Hugo's, made by the claim (7.5.2b-2).
        self::assertSame(6, $this->rowsIn('membership'));
        self::assertSame(1, $this->rowsIn('membership m JOIN "user" u ON u.id = m.user_id WHERE m.member_id IS NOT NULL AND u.email = '.$this->quote(Cast::HUGO)));
        $listed = $this->libraryList($this->listLibraries(Cast::hugoPhone())['body']);
        self::assertCount(1, $listed);
        \assert(\is_array($listed[0]));
        self::assertSame(Cast::GARCIA_LOCAL, $listed[0]['localLibraryId'] ?? null);
        // The cloud wrote that Hugo has an account: his public id, not his email.
        self::assertSame(1, $this->rowsIn('catalog_members WHERE id = '.$this->quote(Cast::id('member:hugo')).' AND account_id IS NOT NULL'));
    }

    public function testKenjiUploadsTheDefaultLibraryAndJhonnyUsesTheSameLocalId(): void
    {
        $withDefaultId = $this->cloudLibrariesOf(Cast::DEFAULT_LOCAL);

        self::assertCount(2, $withDefaultId);
        self::assertSame(2, $this->rowsIn('(SELECT DISTINCT owner_id FROM cloud_library WHERE local_library_id = '.$this->quote(Cast::DEFAULT_LOCAL).') owners'));
        self::assertContains($this->idOfLibraryOwnedBy(Cast::KENJI), $withDefaultId);
        // Neither one holds the other's history.
        self::assertSame(5, $this->operationsIn($this->idOfLibraryOwnedBy(Cast::KENJI)));
        self::assertSame(2, $this->operationsIn(array_first(array_diff($withDefaultId, [$this->idOfLibraryOwnedBy(Cast::KENJI)])) ?? self::fail('Jhonny has no library with the default id.')));
    }

    public function testTheStolenOperationIdsAreInTwoLibraries(): void
    {
        $ana = $this->idOfLibraryOwnedBy(Cast::ANA);
        [$jhonnys] = array_values(array_diff($this->cloudLibrariesOf(Cast::GARCIA_LOCAL), [$ana]));

        $shared = $this->connection->fetchFirstColumn('SELECT operation_id FROM cloud_operation GROUP BY operation_id HAVING COUNT(DISTINCT cloud_library_id) > 1');

        self::assertCount(13, $shared);
        self::assertSame(13, $this->scalar(
            'SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = :library AND operation_id IN (SELECT operation_id FROM cloud_operation WHERE cloud_library_id = :ana)',
            ['library' => $jhonnys, 'ana' => $ana],
        ));
        self::assertSame(17, $this->operationsIn($ana));
    }

    public function testTwoDeviceRowsShareAnId(): void
    {
        $rows = $this->connection->fetchAllAssociative('SELECT d.public_key, u.email FROM device d JOIN "user" u ON u.id = d.user_id WHERE d.device_id = :id ORDER BY u.email', ['id' => Cast::anaPhone()->id]);

        self::assertSame([Cast::ANA, Cast::JHONNY], array_column($rows, 'email'));
        self::assertSame([Cast::anaPhone()->publicKey, Cast::jhonnyAsAnaPhone()->publicKey], array_column($rows, 'public_key'));
    }

    public function testTheSameDeviceIdHasADifferentKeyInEachLibrary(): void
    {
        $keys = $this->connection->fetchAllAssociative(
            'SELECT cloud_library_id, public_key FROM cloud_device_key WHERE device_id = :id',
            ['id' => Cast::anaLaptop()->id],
        );
        $byLibrary = array_column($keys, 'public_key', 'cloud_library_id');

        self::assertCount(3, $byLibrary, 'The Garcías\', Jhonny\'s copy of theirs (stolen operation) and Jhonny\'s default-id library.');
        self::assertSame(Cast::anaLaptop()->publicKey, $byLibrary[$this->idOfLibraryOwnedBy(Cast::ANA)]);
        self::assertContains(Cast::jhonnyAsAnaLaptop()->publicKey, $byLibrary);
    }

    public function testAnOlderOperationOfTheLaptopIsStoredAlongsideANewerOneOfThePhone(): void
    {
        $ana = $this->idOfLibraryOwnedBy(Cast::ANA);
        $hlcs = $this->connection->fetchFirstColumn(
            'SELECT hlc FROM cloud_operation WHERE cloud_library_id = :library ORDER BY hlc',
            ['library' => $ana],
        );
        $older = \sprintf('%016d-%08x-%s', Cast::at(2900), 0, Cast::anaLaptop()->id);
        $newer = \sprintf('%016d-%08x-%s', Cast::at(3000), 0, Cast::anaPhone()->id);

        self::assertContains($older, $hlcs);
        self::assertContains($newer, $hlcs);
    }

    public function testTheGarciaLibraryIsInvisibleToJhonnyThroughTheApi(): void
    {
        $ana = $this->idOfLibraryOwnedBy(Cast::ANA);
        $jhonny = Cast::jhonnyPhone();

        $own = $this->listLibraries($jhonny);
        self::assertCount(3, $this->libraryList($own['body']), 'The one with the Garcías\' local id, the default-id one and the unlinked one; none of Ana\'s.');
        self::assertNotContains($ana, array_column($this->libraryList($own['body']), 'id'));

        foreach (['pull' => ['schemaVersion' => $this->schemaVersion(), 'vector' => new \stdClass()], 'push' => ['operations' => []]] as $route => $body) {
            $response = $this->call('POST', "/v1/cloud/libraries/$ana/$route", $jhonny, $body);
            self::assertSame(Response::HTTP_NOT_FOUND, $response['status']);
            self::assertSame(['error' => 'not_found'], $response['body']);
        }

        self::assertSame(Response::HTTP_OK, $this->call('POST', "/v1/cloud/libraries/$ana/pull", Cast::anaLaptop(), ['schemaVersion' => $this->schemaVersion(), 'vector' => new \stdClass()])['status']);
    }

    public function testTheAttemptsLeftNoRows(): void
    {
        $ana = $this->idOfLibraryOwnedBy(Cast::ANA);
        $unlinked = $this->strings($this->connection->fetchFirstColumn('SELECT id FROM cloud_library WHERE local_library_id IS NULL'));

        self::assertCount(1, $unlinked, 'Jhonny\'s library with no link: created, never filled.');
        self::assertSame(0, $this->operationsIn($unlinked[0]));
        self::assertSame(17, $this->operationsIn($ana));
        self::assertSame(0, $this->operationsWithChange($ana, 'publisher', ['name' => 'Unsigned']));
        self::assertSame(0, $this->operationsWithChange($ana, 'publisher', ['name' => 'Changed after signing']));
        // 17 (the Garcías, with the cloud's registration and its note of Hugo's account) + 5 (Kenji, same) + 13 (their ids in Jhonny's, his link goes by the route) + 2 (Jhonny's default id): nothing of another local id, unsigned or forged got in.
        self::assertSame(37, $this->rowsIn('cloud_operation'));
    }

    public function testLoadingAgainGivesTheSameOperations(): void
    {
        $before = $this->operationFingerprint();

        $this->load();

        self::assertSame($before, $this->operationFingerprint());
        self::assertSame(4, $this->rowsIn('"user"'));
    }

    // ----------------------------------------------------------- helpers ---

    private function load(): void
    {
        $tester = new CommandTester(new Application($this->client->getKernel())->find('doctrine:fixtures:load'));
        $tester->execute([], ['interactive' => false]);
        $tester->assertCommandIsSuccessful();
    }

    /**
     * The operation ids, HLCs and bodies, except the link (its body names the cloud id the server chose) and what the
     * cloud signs itself (its ids and times are the server's, new on every load).
     *
     * @return list<array<string, mixed>>
     */
    private function operationFingerprint(): array
    {
        return $this->connection->fetchAllAssociative(
            'SELECT operation_id, hlc, CAST(body AS TEXT) AS body FROM cloud_operation WHERE body->>\'intent\' <> \'library.link\' AND author_device_id NOT IN (SELECT cloud_device_id::text FROM cloud_library WHERE cloud_device_id IS NOT NULL) ORDER BY operation_id, hlc',
        );
    }

    /** @return array<string, int> */
    private function devicesPerAccount(): array
    {
        $counts = [];
        foreach ($this->connection->fetchAllAssociative('SELECT u.email, COUNT(d.id) AS devices FROM "user" u JOIN device d ON d.user_id = u.id GROUP BY u.email ORDER BY u.email') as $row) {
            \assert(\is_string($row['email']) && \is_int($row['devices']));
            $counts[$row['email']] = $row['devices'];
        }

        return $counts;
    }

    /** @return list<string> */
    private function cloudLibrariesOf(string $local): array
    {
        return $this->strings($this->connection->fetchFirstColumn('SELECT id FROM cloud_library WHERE local_library_id = :local ORDER BY id', ['local' => $local]));
    }

    private function idOfLibraryOwnedBy(string $email): string
    {
        $id = $this->connection->fetchOne('SELECT l.id FROM cloud_library l JOIN "user" u ON u.id = l.owner_id WHERE u.email = :email AND l.local_library_id IS NOT NULL ORDER BY l.created_at LIMIT 1', ['email' => $email]);
        \assert(\is_string($id));

        return $id;
    }

    private function operationsIn(string $library): int
    {
        return $this->scalar('SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = :library', ['library' => $library]);
    }

    /**
     * How many operations of the library have a change of this entity with these fields.
     *
     * @param array<string, mixed> $fields
     */
    private function operationsWithChange(string $library, string $entity, array $fields = []): int
    {
        $pattern = json_encode(['changes' => [['entity' => $entity, 'fields' => (object) $fields]]], \JSON_THROW_ON_ERROR);

        return $this->scalar(
            'SELECT COUNT(*) FROM cloud_operation WHERE cloud_library_id = :library AND body @> CAST(:pattern AS JSONB)',
            ['library' => $library, 'pattern' => $pattern],
        );
    }

    private function rowsIn(string $from): int
    {
        return $this->scalar("SELECT COUNT(*) FROM $from");
    }

    /**
     * @param array<string, mixed> $parameters
     */
    private function scalar(string $sql, array $parameters = []): int
    {
        $value = $this->connection->fetchOne($sql, $parameters);
        \assert(\is_int($value));

        return $value;
    }

    /**
     * @param list<mixed> $values
     *
     * @return list<string>
     */
    private function strings(array $values): array
    {
        return array_map(static function (mixed $value): string {
            \assert(\is_string($value));

            return $value;
        }, $values);
    }

    private function quote(string $value): string
    {
        return $this->connection->quote($value);
    }

    private function schemaVersion(): int
    {
        $version = Spec::json('sync.json')['schemaVersion'] ?? null;
        \assert(\is_int($version));

        return $version;
    }

    /**
     * @return array{status: int, body: array<mixed>}
     */
    private function listLibraries(SigningDevice $as): array
    {
        return $this->call('GET', '/v1/cloud/libraries', $as);
    }

    /**
     * @param array<mixed>|null $body
     *
     * @return array{status: int, body: array<mixed>}
     */
    private function call(string $method, string $uri, SigningDevice $as, ?array $body = null): array
    {
        $this->client->request($method, $uri, server: [
            'CONTENT_TYPE' => 'application/json',
            'HTTP_X-Device-Id' => $as->id,
            'HTTP_Authorization' => 'Bearer '.Cast::token($as),
        ], content: null === $body ? null : json_encode($body, \JSON_THROW_ON_ERROR));
        $content = (string) $this->client->getResponse()->getContent();
        $decoded = '' === $content ? [] : json_decode($content, true, flags: \JSON_THROW_ON_ERROR);
        \assert(\is_array($decoded));

        return ['status' => $this->client->getResponse()->getStatusCode(), 'body' => $decoded];
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
