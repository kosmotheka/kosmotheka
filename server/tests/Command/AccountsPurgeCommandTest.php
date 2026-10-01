<?php

declare(strict_types=1);

namespace App\Tests\Command;

use App\Entity\User;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Console\Application;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;
use Symfony\Component\Console\Tester\CommandTester;

final class AccountsPurgeCommandTest extends KernelTestCase
{
    public function testCommandReportsHowManyRowsItDeleted(): void
    {
        self::bootKernel();
        $entityManager = self::getContainer()->get(EntityManagerInterface::class);

        $user = new User(\sprintf('purge-command-%s@example.com', bin2hex(random_bytes(4))), 'Ada', 'en');
        new \ReflectionProperty($user, 'createdAt')->setValue($user, new \DateTimeImmutable('-25 hours'));
        $entityManager->persist($user);
        $entityManager->flush();

        $application = new Application(self::$kernel);
        $commandTester = new CommandTester($application->find('app:accounts:purge'));
        $commandTester->execute([]);

        $commandTester->assertCommandIsSuccessful();
        self::assertStringContainsString('Deleted 1 unconfirmed account(s)', $commandTester->getDisplay());

        // The purger issues a bulk DQL DELETE, which bypasses the identity
        // map: clear it so find() actually hits the database.
        $entityManager->clear();
        self::assertNull($entityManager->getRepository(User::class)->find($user->getId()));
    }
}
