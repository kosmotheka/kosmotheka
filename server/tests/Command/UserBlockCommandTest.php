<?php

declare(strict_types=1);

namespace App\Tests\Command;

use App\Entity\Device;
use App\Entity\User;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Console\Application;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Tester\CommandTester;

/** D51: blocking an account from the console also signs out all its devices. */
final class UserBlockCommandTest extends KernelTestCase
{
    public function testBlockingSignsOutAllDevicesAndUndoClearsIt(): void
    {
        self::bootKernel();
        $entityManager = self::getContainer()->get(EntityManagerInterface::class);
        $email = \sprintf('block-%s@example.com', bin2hex(random_bytes(4)));
        $user = new User($email, 'Ada', 'en');
        $entityManager->persist($user);
        $device = new Device(
            \sprintf('0196b2f1-0000-7000-8000-%012d', random_int(0, 999_999_999_999)),
            $user,
            'Test device',
            sodium_bin2base64(random_bytes(32), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING),
            hash('sha256', 'whatever'),
        );
        $entityManager->persist($device);
        $entityManager->flush();

        $commandTester = new CommandTester(new Application(self::$kernel)->find('app:user:block'));
        $commandTester->execute(['email' => $email]);
        $commandTester->assertCommandIsSuccessful();

        $entityManager->clear();
        $blocked = $entityManager->getRepository(User::class)->findOneBy(['email' => $email]);
        self::assertNotNull($blocked);
        self::assertTrue($blocked->isBlocked());
        $blockedDevice = $entityManager->getRepository(Device::class)->find($device->getId());
        self::assertNotNull($blockedDevice);
        self::assertFalse($blockedDevice->isSignedIn());

        $commandTester->execute(['email' => $email, '--undo' => true]);
        $commandTester->assertCommandIsSuccessful();

        $entityManager->clear();
        $unblocked = $entityManager->getRepository(User::class)->findOneBy(['email' => $email]);
        self::assertNotNull($unblocked);
        self::assertFalse($unblocked->isBlocked());
    }

    public function testFailsForAnUnknownEmail(): void
    {
        self::bootKernel();

        $commandTester = new CommandTester(new Application(self::$kernel)->find('app:user:block'));
        $commandTester->execute(['email' => 'nobody@example.com']);

        self::assertSame(Command::FAILURE, $commandTester->getStatusCode());
    }
}
