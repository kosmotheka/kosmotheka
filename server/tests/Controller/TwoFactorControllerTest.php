<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Entity\User;
use App\Repository\BackupCodeRepository;
use App\Tests\Support\DeviceAuthTestTrait;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\Clock\Test\ClockSensitiveTrait;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\Mime\Email;

/**
 * Functional coverage of `/v1/account/two-factor/*` (D78, docs/architecture/accounts.md):
 * setup, enabling, disabling and regenerating backup codes.
 */
final class TwoFactorControllerTest extends WebTestCase
{
    use ClockSensitiveTrait;
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string AT = '2026-01-02 12:00:00';
    // One TOTP period later: enabling 2FA already consumes AT's timestep
    // (anti-replay, D78), so a later verify() needs a code from a later one.
    private const string LATER = '2026-01-02 12:00:30';

    /** See AccountControllerTest::createTestClient() for why this happens here, not in setUp(). */
    private function createTestClient(): KernelBrowser
    {
        $client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();

        return $client;
    }

    public function testSetupReturnsASecretAndAnOtpauthUri(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'setup@example.com');

        $client->request('POST', '/v1/account/two-factor/setup', server: $session['headers']);

        self::assertResponseStatusCodeSame(Response::HTTP_OK);
        /** @var array{secret: string, otpauth_uri: string} $body */
        $body = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);
        self::assertNotSame('', $body['secret']);
        self::assertStringStartsWith('otpauth://totp/', $body['otpauth_uri']);
        self::assertStringContainsString('secret='.$body['secret'], $body['otpauth_uri']);
        self::assertStringContainsString('issuer=Kosmotheka', $body['otpauth_uri']);
    }

    public function testSetupWhenAlreadyEnabledFails(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'already-enabled@example.com');
        $this->enableTwoFactor($client, $session['headers']);

        $client->request('POST', '/v1/account/two-factor/setup', server: $session['headers']);

        self::assertResponseStatusCodeSame(Response::HTTP_CONFLICT);
        self::assertSame('{"error":"two_factor_already_enabled"}', $client->getResponse()->getContent());
    }

    public function testSetupWhenTheServerHasTwoFactorOffFails(): void
    {
        self::mockTime(self::AT);
        // .env already loaded TWO_FACTOR_ENABLED into $_ENV for the whole process
        // (tests/bootstrap.php), which EnvVarProcessor reads before $_SERVER or
        // getenv(): overriding only one of them would not be seen.
        $_ENV['TWO_FACTOR_ENABLED'] = 'false';
        $_SERVER['TWO_FACTOR_ENABLED'] = 'false';
        putenv('TWO_FACTOR_ENABLED=false');
        try {
            $client = $this->createTestClient();
            $session = $this->signInNewDevice($client, 'unavailable@example.com');

            $client->request('POST', '/v1/account/two-factor/setup', server: $session['headers']);

            self::assertResponseStatusCodeSame(Response::HTTP_CONFLICT);
            self::assertSame('{"error":"two_factor_unavailable"}', $client->getResponse()->getContent());
        } finally {
            // Restore rather than unset: .env is only loaded once for the whole
            // test process (tests/bootstrap.php), so removing the key entirely
            // would leave every later test unable to resolve it at all.
            $_ENV['TWO_FACTOR_ENABLED'] = 'true';
            $_SERVER['TWO_FACTOR_ENABLED'] = 'true';
            putenv('TWO_FACTOR_ENABLED=true');
        }
    }

    public function testEnableWithoutSetupFails(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'no-setup@example.com');

        $client->request('POST', '/v1/account/two-factor/enable', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode(['code' => '123456'], \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_CONFLICT);
        self::assertSame('{"error":"two_factor_not_started"}', $client->getResponse()->getContent());
    }

    public function testEnableWithAWrongCodeFails(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'wrong-code@example.com');
        $this->setupTwoFactor($client, $session['headers']);

        $client->request('POST', '/v1/account/two-factor/enable', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode(['code' => '000000'], \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_two_factor_code"}', $client->getResponse()->getContent());
    }

    public function testTooManyWrongCodesAreLimitedPerAccount(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'brute-force@example.com');
        $this->setupTwoFactor($client, $session['headers']);

        for ($attempt = 1; $attempt <= 5; ++$attempt) {
            $client->request('POST', '/v1/account/two-factor/enable', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode(['code' => '000000'], \JSON_THROW_ON_ERROR));
            self::assertSame('{"error":"invalid_two_factor_code"}', $client->getResponse()->getContent());
        }

        $client->request('POST', '/v1/account/two-factor/enable', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode(['code' => '000000'], \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_TOO_MANY_REQUESTS);
        self::assertSame('{"error":"too_many_requests"}', $client->getResponse()->getContent());
    }

    public function testEnableWithTheRightCodeReturnsTenBackupCodesAndSendsANotice(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'enable@example.com');

        $backupCodes = $this->enableTwoFactor($client, $session['headers']);

        self::assertCount(10, $backupCodes);
        foreach ($backupCodes as $code) {
            self::assertMatchesRegularExpression('/^[23456789A-HJ-NP-Z]{5}-[23456789A-HJ-NP-Z]{5}$/', $code);
        }

        // The client reboots the kernel before each request, so the message
        // logger only holds messages from the last one: just the notice.
        self::assertEmailCount(1);
        $message = self::getMailerMessage();
        self::assertInstanceOf(Email::class, $message);
        self::assertStringContainsString('enable@example.com', $message->getTo()[0]->getAddress());

        $user = $this->findUser('enable@example.com');
        self::assertNotNull($user);
        self::assertTrue($user->isTwoFactorEnabled());
    }

    public function testACopyOfTheDatabaseDoesNotHoldTheSecretOfTheSecondFactor(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'sealed@example.com');

        $secret = $this->setupTwoFactor($client, $session['headers']);
        self::assertStringNotContainsString($secret, $this->storedSecrets('sealed@example.com'));

        $this->enableWithCode($client, $session['headers'], $secret);
        self::assertResponseIsSuccessful();
        // Whoever has a copy of the database must not be able to make the codes (SEG-063).
        self::assertStringNotContainsString($secret, $this->storedSecrets('sealed@example.com'));
        self::assertNotSame('', $this->storedSecrets('sealed@example.com'));
    }

    public function testTheSealedSecretOfOneAccountIsNoUseInAnother(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $ada = $this->signInNewDevice($client, 'ada-sealed@example.com');
        $grace = $this->signInNewDevice($client, 'grace-sealed@example.com');
        $secret = $this->setupTwoFactor($client, $ada['headers']);
        $this->enableWithCode($client, $ada['headers'], $secret);

        // Someone who can write in the database copies Ada's sealed secret onto Grace's account.
        $this->entityManager()->getConnection()->executeStatement(
            'UPDATE "user" SET totp_secret = (SELECT totp_secret FROM "user" WHERE email = :ada), totp_enabled_at = NOW() WHERE email = :grace',
            ['ada' => 'ada-sealed@example.com', 'grace' => 'grace-sealed@example.com'],
        );

        self::mockTime(self::LATER);
        $client->request('DELETE', '/v1/account/two-factor', server: [...$grace['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode([
            'password' => 'correct horse battery',
            'code' => $this->totpCodeAt($secret, self::LATER),
        ], \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_two_factor_code"}', $client->getResponse()->getContent());
    }

    public function testGetAccountReportsTwoFactorFields(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'account-fields@example.com');
        $this->enableTwoFactor($client, $session['headers']);

        $client->request('GET', '/v1/account', server: $session['headers']);

        /** @var array{two_factor_enabled: bool, backup_codes_left: int} $body */
        $body = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);
        self::assertTrue($body['two_factor_enabled']);
        self::assertSame(10, $body['backup_codes_left']);
    }

    public function testDisableWithAWrongPasswordFails(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'disable-wrong-password@example.com');
        $this->enableTwoFactor($client, $session['headers']);

        $client->request('DELETE', '/v1/account/two-factor', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode(['password' => 'not the right one', 'code' => '123456'], \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_UNAUTHORIZED);
        self::assertSame('{"error":"invalid_credentials"}', $client->getResponse()->getContent());
    }

    public function testDisableWhenNotEnabledFails(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'disable-not-enabled@example.com');

        $client->request('DELETE', '/v1/account/two-factor', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode(['password' => 'correct horse battery', 'code' => '123456'], \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_CONFLICT);
        self::assertSame('{"error":"two_factor_not_enabled"}', $client->getResponse()->getContent());
    }

    public function testDisableWithAWrongCodeFails(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'disable-wrong-code@example.com');
        $this->enableTwoFactor($client, $session['headers']);

        $client->request('DELETE', '/v1/account/two-factor', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode(['password' => 'correct horse battery', 'code' => '000000'], \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
        self::assertSame('{"error":"invalid_two_factor_code"}', $client->getResponse()->getContent());
    }

    public function testDisableWithTheTotpCodeSucceedsAndSendsANotice(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'disable@example.com');
        $secret = $this->setupTwoFactor($client, $session['headers']);
        $this->enableWithCode($client, $session['headers'], $secret);

        self::mockTime(self::LATER);
        $client->request('DELETE', '/v1/account/two-factor', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode([
            'password' => 'correct horse battery',
            'code' => $this->totpCodeAt($secret, self::LATER),
        ], \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);
        // The client reboots the kernel before each request: just the "disabled" notice.
        self::assertEmailCount(1);

        $user = $this->findUser('disable@example.com');
        self::assertNotNull($user);
        self::assertFalse($user->isTwoFactorEnabled());
        self::assertSame(0, $this->backupCodeRepository()->countUnusedByUser($user));
    }

    public function testDisableWithABackupCodeSucceeds(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'disable-backup@example.com');
        $backupCodes = $this->enableTwoFactor($client, $session['headers']);

        $client->request('DELETE', '/v1/account/two-factor', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode([
            'password' => 'correct horse battery',
            'code' => $backupCodes[0],
        ], \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_NO_CONTENT);
    }

    public function testRegenerateBackupCodesReplacesTheOldOnes(): void
    {
        self::mockTime(self::AT);
        $client = $this->createTestClient();
        $session = $this->signInNewDevice($client, 'regenerate@example.com');
        $secret = $this->setupTwoFactor($client, $session['headers']);
        $oldCodes = $this->enableWithCode($client, $session['headers'], $secret);

        self::mockTime(self::LATER);
        $client->request('POST', '/v1/account/two-factor/backup-codes', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode([
            'password' => 'correct horse battery',
            'code' => $this->totpCodeAt($secret, self::LATER),
        ], \JSON_THROW_ON_ERROR));

        self::assertResponseStatusCodeSame(Response::HTTP_OK);
        /** @var array{backup_codes: list<string>} $body */
        $body = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);
        self::assertCount(10, $body['backup_codes']);
        self::assertNotSame($oldCodes, $body['backup_codes']);

        // The old codes no longer disable 2FA.
        $client->request('DELETE', '/v1/account/two-factor', server: [...$session['headers'], 'CONTENT_TYPE' => 'application/json'], content: json_encode([
            'password' => 'correct horse battery',
            'code' => $oldCodes[0],
        ], \JSON_THROW_ON_ERROR));
        self::assertResponseStatusCodeSame(Response::HTTP_BAD_REQUEST);
    }

    /**
     * @param array<string, string> $headers
     *
     * @return array{secret: string, otpauth_uri: string}
     */
    private function setupTwoFactorRaw(KernelBrowser $client, array $headers): array
    {
        $client->request('POST', '/v1/account/two-factor/setup', server: $headers);

        /** @var array{secret: string, otpauth_uri: string} $body */
        $body = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);

        return $body;
    }

    /** @param array<string, string> $headers */
    private function setupTwoFactor(KernelBrowser $client, array $headers): string
    {
        return $this->setupTwoFactorRaw($client, $headers)['secret'];
    }

    /**
     * @param array<string, string> $headers
     *
     * @return list<string>
     */
    private function enableWithCode(KernelBrowser $client, array $headers, string $secret): array
    {
        $client->request('POST', '/v1/account/two-factor/enable', server: [...$headers, 'CONTENT_TYPE' => 'application/json'], content: json_encode([
            'code' => $this->totpCodeAt($secret, self::AT),
        ], \JSON_THROW_ON_ERROR));

        /** @var array{backup_codes: list<string>} $body */
        $body = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);

        return $body['backup_codes'];
    }

    /**
     * @param array<string, string> $headers
     *
     * @return list<string> the backup codes
     */
    private function enableTwoFactor(KernelBrowser $client, array $headers): array
    {
        $secret = $this->setupTwoFactor($client, $headers);

        return $this->enableWithCode($client, $headers, $secret);
    }

    private function totpCodeAt(string $secret, string $at): string
    {
        return self::totpAt($secret, $at);
    }

    private function storedSecrets(string $email): string
    {
        $row = $this->entityManager()->getConnection()->fetchAssociative('SELECT totp_secret, totp_pending_secret FROM "user" WHERE email = :email', ['email' => $email]);
        self::assertIsArray($row);

        return \sprintf('%s|%s', \is_string($row['totp_secret']) ? $row['totp_secret'] : '', \is_string($row['totp_pending_secret']) ? $row['totp_pending_secret'] : '');
    }

    private function findUser(string $email): ?User
    {
        return $this->entityManager()->getRepository(User::class)->findOneBy(['email' => $email]);
    }

    private function backupCodeRepository(): BackupCodeRepository
    {
        return self::getContainer()->get(BackupCodeRepository::class);
    }

    private function entityManager(): EntityManagerInterface
    {
        return self::getContainer()->get(EntityManagerInterface::class);
    }
}
