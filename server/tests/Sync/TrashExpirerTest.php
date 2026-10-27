<?php

declare(strict_types=1);

namespace App\Tests\Sync;

use App\DataFixtures\SigningDevice;
use App\Entity\CloudLibrary;
use App\Sync\SchemaVersion;
use App\Sync\TrashExpirer;
use App\Tests\Support\DeviceAuthTestTrait;
use Doctrine\DBAL\Connection;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\Uid\Uuid;

/**
 * The cloud's safety net for the trash (D105): 60 days from the later of `trashed_at` and when the row reached the
 * server, signed as the cloud's device, applied (never a request), and only what is in the trash.
 */
final class TrashExpirerTest extends WebTestCase
{
    use DeviceAuthTestTrait;

    private const int T = 1_727_000_000_000;
    private const int DAY = 24 * 60 * 60 * 1000;
    private const string ADMIN = '01997c7e-0000-7000-8000-000000000011';
    private const string GIFTS = '01997c7e-0000-7000-8000-0000000000a1';
    private const string TRIPS = '01997c7e-0000-7000-8000-0000000000a2';

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

    public function testWhatHasBeen60DaysInTheTrashSinceTheServerSawItGoesSignedByTheCloud(): void
    {
        $cloud = $this->uploadedFamilyWithATagInTheTrash();
        $library = $this->library($cloud);
        $expirer = self::getContainer()->get(TrashExpirer::class);

        // Sent to the trash in 2024, but the server sees it now: it waits (a wrong clock, a library uploaded late).
        self::assertSame(0, $expirer->expireIn($library));
        self::assertSame([self::GIFTS, self::TRIPS], $this->tags($cloud));

        // Seen 61 days ago: it goes, in one operation of the cloud's device, applied.
        $db = self::getContainer()->get(Connection::class);
        $db->executeStatement('UPDATE catalog_trash_seen SET seen_at = ? WHERE cloud_library_id = ?', [(int) (microtime(true) * 1000) - 61 * self::DAY, $cloud]);
        self::assertSame(1, $expirer->expireIn($this->library($cloud)));
        self::assertSame([self::TRIPS], $this->tags($cloud), 'what is not in the trash stays');
        $last = $db->fetchAssociative(
            "SELECT status, author_device_id, body->>'intent' AS intent FROM cloud_operation WHERE cloud_library_id = ? ORDER BY hlc DESC LIMIT 1",
            [$cloud],
        );
        self::assertSame(['status' => 'applied', 'author_device_id' => $this->identityOf($cloud), 'intent' => 'trash.expire'], $last);
        self::assertSame(0, $db->fetchOne('SELECT count(*)::int FROM catalog_trash_seen WHERE cloud_library_id = ?', [$cloud]), 'forgotten once gone');
    }

    /** The laptop founds the library naming the cloud, adds two tags and sends one to the trash in 2024. */
    private function uploadedFamilyWithATagInTheTrash(): string
    {
        $cloud = $this->createCloud();
        $device = $this->laptop;
        $local = $device->libraryId(SigningDevice::FOUNDING_NONCE);
        $sent = self::T + 4;
        $operations = [
            $device->founding(self::T, self::ADMIN, personName: 'Admin'),
            $device->sign($local, self::T + 1, 'library.link', [['entity' => 'library', 'id' => $local, 'action' => 'update', 'fields' => [
                'cloud_library_id' => $cloud, 'cloud_server' => SigningDevice::SERVER, 'cloud_device_id' => $this->identityOf($cloud),
            ]]]),
            $device->sign($local, self::T + 2, 'tag.add', [
                ['entity' => 'tag', 'id' => self::GIFTS, 'action' => 'create', 'fields' => ['name' => 'regalos']],
                ['entity' => 'tag', 'id' => self::TRIPS, 'action' => 'create', 'fields' => ['name' => 'viajes']],
            ]),
            $device->sign($local, $sent, 'tag.trash', [['entity' => 'tag', 'id' => self::GIFTS, 'action' => 'update', 'fields' => [
                'trashed_at' => gmdate('Y-m-d\TH:i:s', intdiv($sent, 1000)).\sprintf('.%03dZ', $sent % 1000), 'trashed_by' => self::ADMIN,
            ]]]),
        ];
        self::assertSame(['received' => 4, 'known' => 0, 'rejected' => 0], $this->push($cloud, $operations));

        return $cloud;
    }

    private function library(string $cloud): CloudLibrary
    {
        $em = self::getContainer()->get(EntityManagerInterface::class);
        $em->clear();

        return $em->find(CloudLibrary::class, Uuid::fromString($cloud)) ?? self::fail('No library.');
    }

    /** @return list<string> */
    private function tags(string $cloud): array
    {
        $ids = self::getContainer()->get(Connection::class)->fetchFirstColumn('SELECT id FROM catalog_tags WHERE cloud_library_id = ? ORDER BY id', [$cloud]);

        return array_values(array_filter($ids, \is_string(...)));
    }

    private function createCloud(): string
    {
        $id = $this->call('POST', '/v1/cloud/libraries')['id'] ?? null;
        \assert(\is_string($id));

        return $id;
    }

    private function identityOf(string $cloud): string
    {
        $libraries = $this->call('GET', '/v1/cloud/libraries')['libraries'] ?? null;
        \assert(\is_array($libraries));
        foreach ($libraries as $library) {
            if (\is_array($library) && ($library['id'] ?? null) === $cloud && \is_string($library['cloudDeviceId'] ?? null)) {
                return $library['cloudDeviceId'];
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
        $version = self::getContainer()->get(SchemaVersion::class)->current();

        return $this->call('POST', "/v1/cloud/libraries/$cloud/push", [
            'schemaVersion' => $version,
            'operations' => array_map(static fn (array $operation): array => ['schemaVersion' => $version, 'operation' => $operation], $operations),
        ]);
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
