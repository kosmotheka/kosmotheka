<?php

declare(strict_types=1);

namespace App\Tests\Security;

use App\Entity\BackupCode;
use App\Entity\EmailCode;
use App\Entity\User;
use App\Enum\AccountErrorCode;
use App\Enum\EmailCodePurpose;
use App\Repository\EmailCodeRepository;
use App\Security\AccountException;
use App\Security\EmailCodeIssuer;
use App\Security\LoginChallengeIssuer;
use App\Security\TwoFactor;
use Doctrine\ORM\EntityManagerInterface;
use OTPHP\TOTP;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;
use Symfony\Component\Clock\Test\ClockSensitiveTrait;
use Symfony\Component\Lock\LockFactory;
use Symfony\Component\Mime\Email;

/**
 * Two requests with the same single-use code at once (SEG-072). The other request is played by writing to the
 * database behind the entity manager's back, between this request reading the code and using it.
 */
final class SingleUseCodeRaceTest extends KernelTestCase
{
    use ClockSensitiveTrait;

    /** 2026-10-16 10:00:15 UTC: halfway through a 30-second TOTP step. */
    private const int NOW = 1_792_144_815;

    public function testWrongAttemptsCountedByAnotherRequestAreNotLost(): void
    {
        self::bootKernel();
        $user = $this->persistUser();
        self::getContainer()->get(EmailCodeIssuer::class)->issueAndSend($user, EmailCodePurpose::ConfirmEmail);
        $code = $this->liveCode($user);
        $this->entityManager()->createQuery(\sprintf('UPDATE %s c SET c.attempts = 4 WHERE c.id = :id', EmailCode::class))
            ->setParameter('id', $code->getId(), 'uuid')->execute();

        $this->assertRefusedWith(AccountErrorCode::TooManyAttempts, fn () => self::getContainer()->get(EmailCodeIssuer::class)
            ->check($user, EmailCodePurpose::ConfirmEmail, $this->wrongCode()));
    }

    public function testAnEmailCodeSpentByAnotherRequestIsNotSpentAgain(): void
    {
        self::bootKernel();
        $issuer = self::getContainer()->get(EmailCodeIssuer::class);
        $user = $this->persistUser();
        $issuer->issueAndSend($user, EmailCodePurpose::ConfirmEmail);
        $issuer->check($user, EmailCodePurpose::ConfirmEmail, $this->mailedCode());
        $this->entityManager()->createQuery(\sprintf('DELETE FROM %s c WHERE c.id = :id', EmailCode::class))
            ->setParameter('id', $this->liveCode($user)->getId(), 'uuid')->execute();

        $this->assertRefusedWith(AccountErrorCode::InvalidCode, static fn () => $issuer->spend($user, EmailCodePurpose::ConfirmEmail));
    }

    public function testATotpCodeUsedByAnotherRequestIsNotAcceptedAgain(): void
    {
        self::bootKernel();
        $twoFactor = self::getContainer()->get(TwoFactor::class);
        $user = $this->persistUser();
        $totp = $this->totp($twoFactor, $user);
        $twoFactor->enable($user, $totp->at(self::NOW - 30));
        $this->entityManager()->flush();
        $this->entityManager()->createQuery(\sprintf('UPDATE %s u SET u.totpLastTimestep = :step WHERE u.id = :id', User::class))
            ->setParameter('step', intdiv(self::NOW, 30))->setParameter('id', $user->getId(), 'uuid')->execute();

        $this->assertRefusedWith(AccountErrorCode::InvalidTwoFactorCode, static fn () => $twoFactor->verify($user, $totp->at(self::NOW)));
    }

    public function testABackupCodeIsUsedOnceAndItsUseIsWrittenAtOnce(): void
    {
        self::bootKernel();
        $twoFactor = self::getContainer()->get(TwoFactor::class);
        $user = $this->persistUser();
        $backup = $twoFactor->enable($user, $this->totp($twoFactor, $user)->at(self::NOW))[0];
        $this->entityManager()->flush();

        $twoFactor->verify($user, $backup);
        // Not flushed by the caller: the use is in the database already, where another request would look.
        $used = (int) $this->entityManager()->createQuery(\sprintf('SELECT COUNT(b.id) FROM %s b WHERE b.user = :user AND b.usedAt IS NOT NULL', BackupCode::class))
            ->setParameter('user', $user)->getSingleScalarResult();
        self::assertSame(1, $used);
        $this->assertRefusedWith(AccountErrorCode::InvalidTwoFactorCode, static fn () => $twoFactor->verify($user, $backup));
    }

    public function testAChallengeAnotherRequestIsConsumingIsNotValidHere(): void
    {
        self::bootKernel();
        $issuer = self::getContainer()->get(LoginChallengeIssuer::class);
        $challenge = $issuer->issue()['challenge'];
        $held = self::getContainer()->get(LockFactory::class)->createLock(LoginChallengeIssuer::lockKey($challenge));
        self::assertTrue($held->acquire());

        try {
            $this->assertRefusedWith(AccountErrorCode::InvalidChallenge, static fn () => $issuer->consume($challenge));
        } finally {
            $held->release();
        }
    }

    private function assertRefusedWith(AccountErrorCode $expected, callable $attempt): void
    {
        try {
            $attempt();
            self::fail(\sprintf('Expected %s.', $expected->value));
        } catch (AccountException $exception) {
            self::assertSame($expected, $exception->errorCode);
        }
    }

    private function totp(TwoFactor $twoFactor, User $user): TOTP
    {
        $clock = self::mockTime('@'.self::NOW);
        $secret = $twoFactor->startSetup($user)['secret'];
        self::assertNotSame('', $secret);

        return TOTP::createFromSecret($secret, $clock);
    }

    private function liveCode(User $user): EmailCode
    {
        return self::getContainer()->get(EmailCodeRepository::class)->findOneByUserAndPurpose($user, EmailCodePurpose::ConfirmEmail)
            ?? self::fail('No live code.');
    }

    private function mailedCode(): string
    {
        $message = self::getContainer()->get('mailer.message_logger_listener')->getEvents()->getMessages()[0];
        self::assertInstanceOf(Email::class, $message);
        preg_match('/\d{6}/', (string) $message->getTextBody(), $matches);

        return $matches[0] ?? self::fail('No code in the email.');
    }

    private function wrongCode(): string
    {
        return '000000' === $this->mailedCode() ? '000001' : '000000';
    }

    private function persistUser(): User
    {
        $user = new User(\sprintf('race-%s@example.com', bin2hex(random_bytes(4))), 'Ada', 'en');
        $this->entityManager()->persist($user);
        $this->entityManager()->flush();

        return $user;
    }

    private function entityManager(): EntityManagerInterface
    {
        return self::getContainer()->get(EntityManagerInterface::class);
    }
}
