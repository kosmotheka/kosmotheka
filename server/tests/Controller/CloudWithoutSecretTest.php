<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\DataFixtures\SigningDevice;
use App\Entity\CloudLibrary;
use App\Sync\CloudAuthor;
use App\Sync\CloudAuthorException;
use App\Sync\SchemaVersion;
use App\Tests\Support\DeviceAuthTestTrait;
use Doctrine\DBAL\Connection;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\Uid\Uuid;

/**
 * The cloud is optional (D18) also here: a server with an empty CLOUD_KEY_SECRET works as before and its libraries
 * have no cloud device. No exception, no `500`.
 */
final class CloudWithoutSecretTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string LOCAL = '00000000-0000-7000-8000-000000000001';
    private const int T = 1_727_000_000_000;

    private KernelBrowser $client;
    private ?string $secretBefore;

    /** @var array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private array $session;

    protected function setUp(): void
    {
        $this->secretBefore = \is_string($_SERVER['CLOUD_KEY_SECRET'] ?? null) ? $_SERVER['CLOUD_KEY_SECRET'] : null;
        $_SERVER['CLOUD_KEY_SECRET'] = $_ENV['CLOUD_KEY_SECRET'] = '';
        $this->client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();
        $this->session = $this->signInNewDevice($this->client, 'family@example.com');
    }

    protected function tearDown(): void
    {
        if (null === $this->secretBefore) {
            unset($_SERVER['CLOUD_KEY_SECRET'], $_ENV['CLOUD_KEY_SECRET']);
        } else {
            $_SERVER['CLOUD_KEY_SECRET'] = $_ENV['CLOUD_KEY_SECRET'] = $this->secretBefore;
        }
        parent::tearDown();
    }

    public function testCreatingListingLinkingAndUploadingWorkAsBeforeAndTheCloudHasNoDevice(): void
    {
        $created = $this->call('POST', '/v1/cloud/libraries');
        $cloud = $created['id'] ?? null;
        self::assertIsString($cloud);
        self::assertSame(['id' => $cloud, 'cloudDeviceId' => null, 'cloudPublicKey' => null], $created);
        $laptop = SigningDevice::fromSession($this->session);
        $version = self::getContainer()->get(SchemaVersion::class)->current();

        $operations = [$laptop->registration(self::LOCAL, self::T), $laptop->link(self::LOCAL, $cloud, self::T + 1), $laptop->publisher(self::LOCAL, self::T + 2)];
        $linked = $this->call('POST', "/v1/cloud/libraries/$cloud/link", ['schemaVersion' => $version, 'operation' => $operations[1]]);
        $pushed = $this->call('POST', "/v1/cloud/libraries/$cloud/push", ['schemaVersion' => $version, 'operations' => array_map(
            static fn (array $operation): array => ['schemaVersion' => $version, 'operation' => $operation],
            $operations,
        )]);
        $listed = $this->call('GET', '/v1/cloud/libraries');

        self::assertSame(['localLibraryId' => self::LOCAL], $linked);
        self::assertSame(['received' => 3, 'known' => 0, 'rejected' => 0], $pushed);
        self::assertSame([['id' => $cloud, 'localLibraryId' => self::LOCAL, 'cloudDeviceId' => null, 'cloudPublicKey' => null]], $listed);
        $connection = self::getContainer()->get(Connection::class);
        self::assertSame(1, $connection->fetchOne('SELECT COUNT(*) FROM cloud_device_key'), 'only the laptop');
        self::assertSame(3, $connection->fetchOne('SELECT COUNT(*) FROM cloud_operation'), 'the cloud wrote nothing');
        $em = self::getContainer()->get(EntityManagerInterface::class);
        $library = $em->find(CloudLibrary::class, Uuid::fromString($cloud)) ?? self::fail('No library.');
        $this->expectException(CloudAuthorException::class);
        $em->wrapInTransaction(static fn (): string => self::getContainer()->get(CloudAuthor::class)->author($library, 'member.account', []));
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
