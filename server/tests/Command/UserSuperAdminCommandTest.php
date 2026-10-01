<?php

declare(strict_types=1);

namespace App\Tests\Command;

use App\Entity\User;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Console\Application;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Tester\CommandTester;

/** D51: ROLE_SUPER_ADMIN only ever comes from this command. */
final class UserSuperAdminCommandTest extends KernelTestCase
{
    public function testGrantsAndRevokesTheRole(): void
    {
        self::bootKernel();
        $entityManager = self::getContainer()->get(EntityManagerInterface::class);
        $email = \sprintf('super-admin-%s@example.com', bin2hex(random_bytes(4)));
        $user = new User($email, 'Ada', 'en');
        $entityManager->persist($user);
        $entityManager->flush();

        $commandTester = $this->commandTester();
        $commandTester->execute(['email' => $email]);
        $commandTester->assertCommandIsSuccessful();

        $entityManager->clear();
        $granted = $entityManager->getRepository(User::class)->findOneBy(['email' => $email]);
        self::assertNotNull($granted);
        self::assertContains('ROLE_SUPER_ADMIN', $granted->getRoles());

        $commandTester->execute(['email' => $email, '--revoke' => true]);
        $commandTester->assertCommandIsSuccessful();

        $entityManager->clear();
        $revoked = $entityManager->getRepository(User::class)->findOneBy(['email' => $email]);
        self::assertNotNull($revoked);
        self::assertNotContains('ROLE_SUPER_ADMIN', $revoked->getRoles());
    }

    public function testFailsForAnUnknownEmail(): void
    {
        self::bootKernel();

        $commandTester = $this->commandTester();
        $commandTester->execute(['email' => 'nobody@example.com']);

        self::assertSame(Command::FAILURE, $commandTester->getStatusCode());
    }

    private function commandTester(): CommandTester
    {
        return new CommandTester(new Application(self::$kernel)->find('app:user:super-admin'));
    }
}
