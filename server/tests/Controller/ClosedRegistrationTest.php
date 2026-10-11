<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Entity\User;
use App\Security\RegistrationGate;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;

/**
 * A server whose host only lets some addresses make an account (`REGISTRATION_ALLOWED_EMAILS`): everybody else gets
 * `registration_closed` and leaves nothing behind. With the list empty, registration is open, as it always was.
 */
final class ClosedRegistrationTest extends WebTestCase
{
    use MailerAssertionsTrait;

    private const string VARIABLE = 'REGISTRATION_ALLOWED_EMAILS';

    /** Put back as `.env` has it: without the variable the next test's kernel could not resolve it at all. */
    protected function tearDown(): void
    {
        $_ENV[self::VARIABLE] = $_SERVER[self::VARIABLE] = '';
        parent::tearDown();
    }

    public function testAnAddressOutsideTheListMakesNoAccountAndGetsNoEmail(): void
    {
        $client = $this->clientAllowing('ada@example.com, hugo@example.com');

        $this->register($client, 'jhonny@example.com');

        self::assertResponseStatusCodeSame(403);
        self::assertSame('registration_closed', $this->body($client)['error'] ?? null);
        self::assertEmailCount(0);
        self::assertNull($this->findUser('jhonny@example.com'));
    }

    public function testAnAddressOnTheListRegistersHoweverItIsTyped(): void
    {
        $client = $this->clientAllowing(' Ada@Example.com ,hugo@example.com');

        $this->register($client, 'ADA@example.COM');

        self::assertResponseStatusCodeSame(202);
        self::assertEmailCount(1);
        self::assertNotNull($this->findUser('ada@example.com'));
    }

    public function testCapitalsSpacesAndLookAlikesDoNotGetAroundTheList(): void
    {
        $client = $this->clientAllowing('ada@example.com');

        foreach (['ada@example.com.evil.test', 'xada@example.com', 'ada@example.comx', 'ada+1@example.com', 'ada@example.com,jhonny@example.com'] as $email) {
            $this->register($client, $email);
            self::assertNotSame(202, $client->getResponse()->getStatusCode(), $email);
        }
        self::assertEmailCount(0);
        self::assertSame(0, $this->entityManager()->getRepository(User::class)->count([]));
    }

    public function testWithAnEmptyListAnybodyRegisters(): void
    {
        foreach (['', ' , '] as $list) {
            self::ensureKernelShutdown();
            $client = $this->clientAllowing($list);
            $this->register($client, 'anybody@example.com');
            self::assertResponseStatusCodeSame(202);
        }
    }

    public function testTheListIsNeverPublished(): void
    {
        $client = $this->clientAllowing('ada@example.com');

        $client->request('GET', '/v1/server');
        self::assertStringNotContainsString('ada@example.com', (string) $client->getResponse()->getContent());
        self::assertTrue($this->body($client)['registration_restricted'] ?? null);

        $this->register($client, 'jhonny@example.com');
        self::assertStringNotContainsString('ada@example.com', (string) $client->getResponse()->getContent());
    }

    public function testAnOpenServerSaysRegistrationIsNotRestricted(): void
    {
        $client = $this->clientAllowing('');

        $client->request('GET', '/v1/server');

        self::assertFalse($this->body($client)['registration_restricted'] ?? null);
    }

    public function testTheGateReadsItsList(): void
    {
        self::assertTrue(new RegistrationGate('')->allows('anybody@example.com'));
        self::assertFalse(new RegistrationGate('')->isRestricted());
        self::assertTrue(new RegistrationGate("ada@example.com;hugo@example.com\n")->allows('hugo@example.com'));
        self::assertFalse(new RegistrationGate('ada@example.com')->allows('hugo@example.com'));
    }

    private function clientAllowing(string $list): KernelBrowser
    {
        $_ENV[self::VARIABLE] = $_SERVER[self::VARIABLE] = $list;
        $client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();

        return $client;
    }

    private function register(KernelBrowser $client, string $email): void
    {
        $client->request('POST', '/v1/account/register', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode([
            'email' => $email,
            'password' => 'correct horse battery',
            'name' => 'Somebody',
            'locale' => 'en',
        ], \JSON_THROW_ON_ERROR));
    }

    /** @return array<mixed> */
    private function body(KernelBrowser $client): array
    {
        $decoded = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);
        \assert(\is_array($decoded));

        return $decoded;
    }

    private function findUser(string $email): ?User
    {
        return $this->entityManager()->getRepository(User::class)->findOneBy(['email' => $email]);
    }

    private function entityManager(): EntityManagerInterface
    {
        return self::getContainer()->get(EntityManagerInterface::class);
    }
}
