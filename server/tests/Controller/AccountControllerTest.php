<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Entity\User;
use App\Enum\EmailCodePurpose;
use App\Repository\EmailCodeRepository;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\Mime\Email;

/**
 * Functional coverage of `/v1/account/*` (docs/architecture/accounts.md):
 * registration never reveals whether an email exists, codes expire, allow 5
 * wrong attempts, and are single use.
 */
final class AccountControllerTest extends WebTestCase
{
    use MailerAssertionsTrait;

    /**
     * WebTestCase forbids booting the kernel before createClient(), so the
     * rate limiter cache (filesystem, not rolled back by dama/doctrine-test-
     * bundle like the database) is cleared here, right after, instead of in
     * setUp(). Every test starts from this instead of createClient() directly.
     */
    private function createTestClient(): KernelBrowser
    {
        $client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();

        return $client;
    }

    public function testRegisterNewEmailSendsAConfirmationCode(): void
    {
        $client = $this->createTestClient();

        $this->register($client, 'new@example.com', 'Ada', 'en');

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertSame('{}', $client->getResponse()->getContent());
        self::assertEmailCount(1);

        $message = self::getMailerMessage();
        self::assertInstanceOf(Email::class, $message);
        self::assertStringContainsString('new@example.com', $message->getTo()[0]->getAddress());

        $user = $this->findUser('new@example.com');
        self::assertNotNull($user);
        self::assertFalse($user->isConfirmed());
    }

    public function testRegisterExistingUnconfirmedUserUpdatesItAndResendsACode(): void
    {
        $client = $this->createTestClient();
        $this->register($client, 'pending@example.com', 'Old Name', 'en');

        $this->register($client, 'pending@example.com', 'New Name', 'es', 'another good password');

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertEmailCount(1);

        $user = $this->findUser('pending@example.com');
        self::assertNotNull($user);
        self::assertSame('New Name', $user->getName());
        self::assertSame('es', $user->getLocale());
    }

    public function testRegisterExistingConfirmedUserChangesNothingAndSendsANotice(): void
    {
        $client = $this->createTestClient();
        $email = $this->registerAndConfirm($client, 'confirmed@example.com');

        $this->register($client, $email, 'Someone else', 'en', 'a different password');

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertEmailCount(1);

        $user = $this->findUser($email);
        self::assertNotNull($user);
        self::assertSame('Ada', $user->getName());
    }

    public function testRegisterInSpanishSendsTheEmailInSpanish(): void
    {
        $client = $this->createTestClient();

        $this->register($client, 'es-speaker@example.com', 'Ada', 'es');

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);

        $message = self::getMailerMessage();
        self::assertInstanceOf(Email::class, $message);
        self::assertStringContainsString('Confirma tu cuenta', $message->getSubject() ?? '');
    }

    /**
     * @param array<string, mixed> $payload
     */
    #[DataProvider('invalidRegistrationPayloads')]
    public function testRegisterRejectsInvalidPayloads(array $payload): void
    {
        $client = $this->createTestClient();

        $client->request('POST', '/v1/account/register', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode($payload, \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_UNPROCESSABLE_ENTITY);
    }

    /**
     * @return iterable<string, array{array<string, mixed>}>
     */
    public static function invalidRegistrationPayloads(): iterable
    {
        $valid = ['email' => 'valid@example.com', 'password' => 'a long enough password', 'name' => 'Ada', 'locale' => 'en'];

        yield 'invalid email' => [[...$valid, 'email' => 'not-an-email']];
        yield 'short password' => [[...$valid, 'password' => 'short']];
        yield 'blank name' => [[...$valid, 'name' => ' ']];
        yield 'unknown locale' => [[...$valid, 'locale' => 'fr']];
    }

    public function testRegisterIsRateLimitedPerIp(): void
    {
        $client = $this->createTestClient();

        for ($i = 0; $i < 5; ++$i) {
            $this->register($client, \sprintf('ip-limit-%d@example.com', $i), 'Ada', 'en');
            self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        }

        $this->register($client, 'ip-limit-5@example.com', 'Ada', 'en');

        self::assertResponseStatusCodeSame(Response::HTTP_TOO_MANY_REQUESTS);
        self::assertSame('{"error":"too_many_requests"}', $client->getResponse()->getContent());
        self::assertTrue($client->getResponse()->headers->has('Retry-After'));
    }

    public function testConfirmWithTheRightCodeConfirmsTheAccount(): void
    {
        $client = $this->createTestClient();
        $email = 'confirm-me@example.com';
        $code = $this->registerAndGetCode($client, $email);

        $this->confirm($client, $email, $code);

        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        $user = $this->findUser($email);
        self::assertNotNull($user);
        self::assertTrue($user->isConfirmed());
    }

    public function testConfirmWithAWrongCodeFails(): void
    {
        $client = $this->createTestClient();
        $email = 'wrong-code@example.com';
        $this->registerAndGetCode($client, $email);

        $this->confirm($client, $email, '000000');

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_code"}', $client->getResponse()->getContent());
    }

    public function testConfirmWithAnExpiredCodeFails(): void
    {
        $client = $this->createTestClient();
        $email = 'expired-code@example.com';
        $code = $this->registerAndGetCode($client, $email);

        $user = $this->findUser($email);
        self::assertNotNull($user);
        $emailCode = self::getContainer()->get(EmailCodeRepository::class)->findOneByUserAndPurpose($user, EmailCodePurpose::ConfirmEmail);
        self::assertNotNull($emailCode);
        new \ReflectionProperty($emailCode, 'expiresAt')->setValue($emailCode, new \DateTimeImmutable('-1 minute'));
        $this->entityManager()->flush();

        $this->confirm($client, $email, $code);

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"code_expired"}', $client->getResponse()->getContent());
    }

    public function testConfirmLocksAfterFiveWrongAttempts(): void
    {
        $client = $this->createTestClient();
        $email = 'five-attempts@example.com';
        $this->registerAndGetCode($client, $email);

        for ($i = 0; $i < 4; ++$i) {
            $this->confirm($client, $email, '000000');
            self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
            self::assertSame('{"error":"invalid_code"}', $client->getResponse()->getContent());
        }

        $this->confirm($client, $email, '000000');

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"too_many_attempts"}', $client->getResponse()->getContent());
    }

    public function testConfirmCannotReuseAUsedCode(): void
    {
        $client = $this->createTestClient();
        $email = 'reuse@example.com';
        $code = $this->registerAndGetCode($client, $email);

        $this->confirm($client, $email, $code);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        $this->confirm($client, $email, $code);
        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_code"}', $client->getResponse()->getContent());
    }

    public function testConfirmOfAnAlreadyConfirmedAccountFails(): void
    {
        $client = $this->createTestClient();
        $email = $this->registerAndConfirm($client, 'already-confirmed@example.com');

        $this->confirm($client, $email, '123456');

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_code"}', $client->getResponse()->getContent());
    }

    public function testConfirmOfAnUnknownEmailFails(): void
    {
        $client = $this->createTestClient();

        $this->confirm($client, 'nobody@example.com', '123456');

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_code"}', $client->getResponse()->getContent());
    }

    public function testRequestCodeResendsOnlyForAnUnconfirmedUser(): void
    {
        $client = $this->createTestClient();
        $this->register($client, 'resend@example.com', 'Ada', 'en');

        $this->requestCode($client, 'resend@example.com');

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertEmailCount(1);
    }

    public function testRequestCodeForAConfirmedUserSendsNothing(): void
    {
        $client = $this->createTestClient();
        $email = $this->registerAndConfirm($client, 'already-done@example.com');

        $this->requestCode($client, $email);

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertEmailCount(0);
    }

    public function testRequestCodeForAnUnknownEmailSendsNothing(): void
    {
        $client = $this->createTestClient();

        $this->requestCode($client, 'unknown@example.com');

        self::assertResponseStatusCodeSame(Response::HTTP_ACCEPTED);
        self::assertEmailCount(0);
    }

    public function testRequestCodeRejectsAnUnsupportedPurpose(): void
    {
        $client = $this->createTestClient();

        $client->request('POST', '/v1/account/code', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode(['email' => 'someone@example.com', 'purpose' => 'reset_password'], \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_UNPROCESSABLE_ENTITY);
    }

    private function register(KernelBrowser $client, string $email, string $name, string $locale, string $password = 'correct horse battery'): void
    {
        $client->request('POST', '/v1/account/register', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode([
            'email' => $email,
            'password' => $password,
            'name' => $name,
            'locale' => $locale,
        ], \JSON_THROW_ON_ERROR));
    }

    private function confirm(KernelBrowser $client, string $email, string $code): void
    {
        $client->request('POST', '/v1/account/confirm', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode(['email' => $email, 'code' => $code], \JSON_THROW_ON_ERROR));
    }

    private function requestCode(KernelBrowser $client, string $email, string $purpose = 'confirm_email'): void
    {
        $client->request('POST', '/v1/account/code', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode(['email' => $email, 'purpose' => $purpose], \JSON_THROW_ON_ERROR));
    }

    private function registerAndGetCode(KernelBrowser $client, string $email): string
    {
        $this->register($client, $email, 'Ada', 'en');

        $message = self::getMailerMessage();
        self::assertInstanceOf(Email::class, $message);
        $body = (string) $message->getTextBody();
        self::assertMatchesRegularExpression('/\d{6}/', $body, 'The email should contain a 6-digit code.');
        preg_match('/\d{6}/', $body, $matches);

        return $matches[0];
    }

    private function registerAndConfirm(KernelBrowser $client, string $email): string
    {
        $code = $this->registerAndGetCode($client, $email);
        $this->confirm($client, $email, $code);
        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);

        return $email;
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
