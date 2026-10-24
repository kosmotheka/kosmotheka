<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\DataFixtures\SigningDevice;
use App\Entity\CloudCover;
use App\Entity\CloudLibrary;
use App\Entity\Membership;
use App\Entity\User;
use App\Security\AccountPurger;
use App\Sync\CloudCovers;
use App\Tests\Support\DeviceAuthTestTrait;
use App\Tests\Support\Spec;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\Clock\Test\ClockSensitiveTrait;
use Symfony\Component\Filesystem\Filesystem;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\Uid\Uuid;

/**
 * The cover files of a cloud library (D82, part 3): the swap, and the hostile cases first. Jhonny has an account of
 * his own and knows the family's ids (D84); the server is no storage for files no edition of the library uses.
 */
final class CloudCoversTest extends WebTestCase
{
    use ClockSensitiveTrait;
    use DeviceAuthTestTrait;

    private const string FOUNDER = '01997c7e-0000-7000-8000-0000000000f1';
    private const int T = 1_727_000_000_000;
    private const string WORK = '01997c7e-0000-7000-8000-0000000000a1';
    private const string EDITION = '01997c7e-0000-7000-8000-0000000000e1';

    private KernelBrowser $client;

    /** @var array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private array $family;

    /** @var array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private array $jhonny;

    private SigningDevice $laptop;
    private string $local;
    private int $time = self::T;

    protected function setUp(): void
    {
        $this->client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();
        new Filesystem()->remove($this->coversDir());

        $this->family = $this->signInNewDevice($this->client, 'family@example.com');
        $this->jhonny = $this->signInNewDevice($this->client, 'jhonny@example.com');
        $this->laptop = SigningDevice::fromSession($this->family);
        $this->local = $this->laptop->libraryId();
    }

    // ------------------------------------------------------------ normal ---

    public function testACoverOfTheLibraryGoesUpOnceAndComesDownTheSame(): void
    {
        [$name, $bytes] = $this->webp('catan');
        $cloud = $this->libraryWithCover($name);

        self::assertSame(['send' => [], 'want' => [$name]], $this->inventory($this->family, $cloud, [$name])['body']);
        self::assertSame(Response::HTTP_NO_CONTENT, $this->put($this->family, $cloud, $name, $bytes));
        self::assertSame(Response::HTTP_NO_CONTENT, $this->put($this->family, $cloud, $name, $bytes), 'sent twice: kept once');

        self::assertSame(['send' => [$name], 'want' => []], $this->inventory($this->family, $cloud, [])['body']);
        self::assertSame(['send' => [], 'want' => []], $this->inventory($this->family, $cloud, [$name])['body']);
        $this->client->request('GET', "/v1/cloud/libraries/$cloud/covers/$name", server: $this->family['headers']);
        self::assertSame(Response::HTTP_OK, $this->client->getResponse()->getStatusCode());
        self::assertSame($bytes, $this->client->getInternalResponse()->getContent());
        self::assertSame(1, $this->rowsOf(CloudCover::class));
    }

    public function testAFileLostFromTheDiskIsAskedForAgain(): void
    {
        [$name, $bytes] = $this->webp('catan');
        $cloud = $this->libraryWithCover($name);
        $this->put($this->family, $cloud, $name, $bytes);

        new Filesystem()->remove($this->coversDir());

        self::assertSame(['send' => [], 'want' => [$name]], $this->inventory($this->family, $cloud, [$name])['body']);
        self::assertSame(Response::HTTP_NO_CONTENT, $this->put($this->family, $cloud, $name, $bytes));
        self::assertFileExists($this->coversDir()."/$cloud/$name");
    }

    // ----------------------------------------------------------- hostile ---

    public function testAnAccountWithoutTheLibraryGetsAndGivesNothing(): void
    {
        [$name, $bytes] = $this->webp('catan');
        $cloud = $this->libraryWithCover($name);
        $this->put($this->family, $cloud, $name, $bytes);

        self::assertSame(Response::HTTP_NOT_FOUND, $this->inventory($this->jhonny, $cloud, [])['status']);
        $this->client->request('GET', "/v1/cloud/libraries/$cloud/covers/$name", server: $this->jhonny['headers']);
        self::assertSame(Response::HTTP_NOT_FOUND, $this->client->getResponse()->getStatusCode());
        [$other, $otherBytes] = $this->webp('jhonny');
        self::assertSame(Response::HTTP_NOT_FOUND, $this->put($this->jhonny, $cloud, $other, $otherBytes));
    }

    public function testAMemberWhoseAccessWasWithdrawnGetsNoCover(): void
    {
        [$name, $bytes] = $this->webp('catan');
        $cloud = $this->libraryWithCover($name);
        $this->put($this->family, $cloud, $name, $bytes);
        $this->withdrawFamily($cloud);

        $this->client->request('GET', "/v1/cloud/libraries/$cloud/covers/$name", server: $this->family['headers']);

        self::assertNotSame(Response::HTTP_OK, $this->client->getResponse()->getStatusCode());
        self::assertNotSame($bytes, $this->client->getInternalResponse()->getContent());
    }

    public function testTheServerKeepsNoFileThatNoEditionOfTheLibraryUses(): void
    {
        [$name] = $this->webp('catan');
        $cloud = $this->libraryWithCover($name);
        [$other, $otherBytes] = $this->webp('anything');

        self::assertSame(['send' => [], 'want' => []], $this->inventory($this->family, $cloud, [$other])['body']);
        self::assertSame(Response::HTTP_NOT_FOUND, $this->put($this->family, $cloud, $other, $otherBytes));
        self::assertSame(0, $this->rowsOf(CloudCover::class));
        self::assertDirectoryDoesNotExist($this->coversDir()."/$cloud");
    }

    public function testBytesThatAreNotTheNamedCoverAreRefused(): void
    {
        [$name] = $this->webp('catan');
        $cloud = $this->libraryWithCover($name);
        [, $otherBytes] = $this->webp('another');

        self::assertSame(Response::HTTP_UNPROCESSABLE_ENTITY, $this->put($this->family, $cloud, $name, $otherBytes));
        self::assertSame(0, $this->rowsOf(CloudCover::class));
    }

    public function testAFileThatIsNotAnImageOfItsTypeIsRefused(): void
    {
        $text = 'not an image, whatever its name says';
        $name = hash('sha256', $text).'.webp';
        $cloud = $this->libraryWithCover($name);

        self::assertSame(Response::HTTP_UNPROCESSABLE_ENTITY, $this->put($this->family, $cloud, $name, $text));
        self::assertSame(0, $this->rowsOf(CloudCover::class));
    }

    public function testANameThatIsNotACoverNameNeverReachesTheDisk(): void
    {
        $cloud = $this->libraryWithCover($this->webp('catan')[0]);

        foreach (['..%2F..%2F.env.local', 'cover.webp', str_repeat('A', 64).'.webp', str_repeat('a', 64).'.php'] as $name) {
            self::assertSame(Response::HTTP_NOT_FOUND, $this->put($this->family, $cloud, $name, 'x'), $name);
        }
        $this->client->request('POST', "/v1/cloud/libraries/$cloud/covers/inventory", server: ['CONTENT_TYPE' => 'application/json'] + $this->family['headers'], content: '{"have":["../secret"]}');
        self::assertSame(Response::HTTP_UNPROCESSABLE_ENTITY, $this->client->getResponse()->getStatusCode());
    }

    public function testALibraryDoesNotGoOverItsRoomForCovers(): void
    {
        [$name, $bytes] = $this->webp('catan');
        $cloud = $this->libraryWithCover($name);
        $em = self::getContainer()->get(EntityManagerInterface::class);
        $library = $em->find(CloudLibrary::class, Uuid::fromString($cloud)) ?? self::fail('No library.');
        $em->persist(new CloudCover($library, str_repeat('b', 64).'.webp', CloudCovers::MAX_PER_LIBRARY - 10, new \DateTimeImmutable()));
        $em->flush();

        self::assertSame(Response::HTTP_REQUEST_ENTITY_TOO_LARGE, $this->put($this->family, $cloud, $name, $bytes));
    }

    // ----------------------------------------------------------- cleanup ---

    public function testACoverNobodyUsesIsRemovedADayLater(): void
    {
        $clock = self::mockTime('2026-10-24 10:00:00');
        [$name, $bytes] = $this->webp('catan');
        $cloud = $this->libraryWithCover($name);
        $this->put($this->family, $cloud, $name, $bytes);
        $this->push($cloud, [$this->laptop->sign($this->local, $this->next(), 'edition.update', [[
            'entity' => 'edition', 'id' => self::EDITION, 'action' => 'update', 'fields' => ['cover' => null],
        ]])]);
        $purger = self::getContainer()->get(AccountPurger::class);

        self::assertSame(0, $purger->purge()['covers'], 'not at once');
        $clock->sleep(25 * 3600);
        self::assertSame(1, $purger->purge()['covers']);
        self::assertSame(0, $this->rowsOf(CloudCover::class));
        self::assertFileDoesNotExist($this->coversDir()."/$cloud/$name");
    }

    // ----------------------------------------------------------- helpers ---
    /** @return array{0: string, 1: string} a cover's name and bytes: a WebP by its first bytes, different for each seed */
    private function webp(string $seed): array
    {
        $bytes = 'RIFF'.pack('V', 32).'WEBPVP8 '.hash('sha256', $seed, true);

        return [hash('sha256', $bytes).'.webp', $bytes];
    }

    /** A cloud library uploaded by the laptop, with one book whose edition points to `$cover`. */
    private function libraryWithCover(string $cover): string
    {
        $id = $this->call('POST', '/v1/cloud/libraries', $this->family)['body']['id'] ?? null;
        \assert(\is_string($id));
        // Signed in the order they happen: each one names the one before it (D87).
        $founding = $this->laptop->founding($this->next(), self::FOUNDER);
        $link = $this->laptop->link($this->local, $id, $this->next());
        $book = $this->laptop->sign($this->local, $this->next(), 'book.add', [
            ['entity' => 'work', 'id' => self::WORK, 'action' => 'create', 'fields' => ['kind' => 'boardgame', 'title' => 'Catan']],
            ['entity' => 'edition', 'id' => self::EDITION, 'action' => 'create', 'fields' => ['work_id' => self::WORK, 'title' => 'Catan', 'cover' => $cover]],
        ]);
        $this->push($id, [$founding, $link, $book]);

        return $id;
    }

    /** @param list<array<string, mixed>> $operations */
    private function push(string $cloud, array $operations): void
    {
        $version = Spec::json('sync.json')['schemaVersion'] ?? null;
        \assert(\is_int($version));
        $response = $this->call('POST', "/v1/cloud/libraries/$cloud/push", $this->family, [
            'schemaVersion' => $version,
            'operations' => array_map(static fn (array $operation): array => ['schemaVersion' => $version, 'operation' => $operation], $operations),
        ]);
        self::assertSame(\count($operations), $response['body']['received'] ?? null, (string) json_encode($response['body']));
        self::assertSame(0, $response['body']['rejected'] ?? null);
    }

    private function next(): int
    {
        return ++$this->time;
    }

    private function withdrawFamily(string $cloud): void
    {
        $em = self::getContainer()->get(EntityManagerInterface::class);
        $user = $em->getRepository(User::class)->findOneBy(['email' => 'family@example.com']) ?? self::fail('No family.');
        $library = $em->find(CloudLibrary::class, Uuid::fromString($cloud)) ?? self::fail('No library.');
        $membership = $em->getRepository(Membership::class)->findOneBy(['user' => $user, 'cloudLibrary' => $library]) ?? self::fail('No membership.');
        $membership->withdraw(new \DateTimeImmutable());
        $em->flush();
    }

    /**
     * @param array{headers: array<string, string>} $session
     * @param list<string>                          $have
     *
     * @return array{status: int, body: array<mixed>}
     */
    private function inventory(array $session, string $cloud, array $have): array
    {
        return $this->call('POST', "/v1/cloud/libraries/$cloud/covers/inventory", $session, ['have' => $have]);
    }

    /** @param array{headers: array<string, string>} $session */
    private function put(array $session, string $cloud, string $name, string $bytes): int
    {
        $this->client->request('PUT', "/v1/cloud/libraries/$cloud/covers/$name", server: ['CONTENT_TYPE' => 'application/octet-stream'] + $session['headers'], content: $bytes);

        return $this->client->getResponse()->getStatusCode();
    }

    /**
     * @param array{headers: array<string, string>}|null $session
     * @param array<string, mixed>|null                  $body
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

    /** @param class-string $entity */
    private function rowsOf(string $entity): int
    {
        return self::getContainer()->get(EntityManagerInterface::class)->getRepository($entity)->count([]);
    }

    private function coversDir(): string
    {
        return (string) self::getContainer()->getParameter('app.covers_dir');
    }
}
