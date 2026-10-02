<?php

declare(strict_types=1);

namespace App\Tests\Command;

use App\Entity\BackupCode;
use App\Entity\User;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Console\Application;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Tester\CommandTester;

/** D78: for someone who lost both their phone and their backup codes. */
final class UserTwoFactorDisableCommandTest extends KernelTestCase
{
    public function testRemovesTheSecretAndItsBackupCodes(): void
    {
        self::bootKernel();
        $entityManager = self::getContainer()->get(EntityManagerInterface::class);
        $email = \sprintf('two-factor-disable-%s@example.com', bin2hex(random_bytes(4)));
        $user = new User($email, 'Ada', 'en');
        $user->setTotpSecret('JBSWY3DPEHPK3PXP');
        $user->setTotpEnabledAt(new \DateTimeImmutable());
        $entityManager->persist($user);
        $entityManager->persist(new BackupCode($user, hash_hmac('sha256', 'ABCDE12345', 'whatever')));
        $entityManager->flush();

        $commandTester = new CommandTester(new Application(self::$kernel)->find('app:user:two-factor-disable'));
        $commandTester->execute(['email' => $email]);
        $commandTester->assertCommandIsSuccessful();

        $entityManager->clear();
        $updated = $entityManager->getRepository(User::class)->findOneBy(['email' => $email]);
        self::assertNotNull($updated);
        self::assertFalse($updated->isTwoFactorEnabled());
        self::assertSame([], $entityManager->getRepository(BackupCode::class)->findBy(['user' => $updated]));
    }

    public function testFailsForAnUnknownEmail(): void
    {
        self::bootKernel();

        $commandTester = new CommandTester(new Application(self::$kernel)->find('app:user:two-factor-disable'));
        $commandTester->execute(['email' => 'nobody@example.com']);

        self::assertSame(Command::FAILURE, $commandTester->getStatusCode());
    }

    public function testFailsWhenTwoFactorIsNotEnabled(): void
    {
        self::bootKernel();
        $entityManager = self::getContainer()->get(EntityManagerInterface::class);
        $email = \sprintf('two-factor-not-enabled-%s@example.com', bin2hex(random_bytes(4)));
        $entityManager->persist(new User($email, 'Ada', 'en'));
        $entityManager->flush();

        $commandTester = new CommandTester(new Application(self::$kernel)->find('app:user:two-factor-disable'));
        $commandTester->execute(['email' => $email]);

        self::assertSame(Command::FAILURE, $commandTester->getStatusCode());
    }
}
