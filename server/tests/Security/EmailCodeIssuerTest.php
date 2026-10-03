<?php

declare(strict_types=1);

namespace App\Tests\Security;

use App\Entity\User;
use App\Enum\EmailCodePurpose;
use App\Repository\EmailCodeRepository;
use App\Security\AccountException;
use App\Security\EmailCodeIssuer;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;
use Symfony\Component\Mime\Email;

/**
 * Exercises the issuer through the container, the way the controller does,
 * and checks the stored hash is never the plain code (D49).
 */
final class EmailCodeIssuerTest extends KernelTestCase
{
    public function testIssuedCodeHashesDifferFromThePlainCode(): void
    {
        self::bootKernel();
        $container = self::getContainer();

        $user = $this->persistUser($container->get(EntityManagerInterface::class));
        $container->get(EmailCodeIssuer::class)->issueAndSend($user, EmailCodePurpose::ConfirmEmail);

        $emailCode = $container->get(EmailCodeRepository::class)->findOneByUserAndPurpose($user, EmailCodePurpose::ConfirmEmail);
        self::assertNotNull($emailCode);

        $message = $container->get('mailer.message_logger_listener')->getEvents()->getMessages()[0];
        self::assertInstanceOf(Email::class, $message);
        preg_match('/\d{6}/', (string) $message->getTextBody(), $matches);
        $code = $matches[0] ?? self::fail('No code in the email.');

        self::assertNotSame($code, $emailCode->getCodeHash());
        self::assertSame(64, \strlen($emailCode->getCodeHash()), 'SHA-256 hex is 64 characters.');
    }

    public function testVerifyAcceptsTheRightCodeOnceAndRejectsItAfterwards(): void
    {
        self::bootKernel();
        $container = self::getContainer();
        $entityManager = $container->get(EntityManagerInterface::class);
        $issuer = $container->get(EmailCodeIssuer::class);

        $user = $this->persistUser($entityManager);
        $issuer->issueAndSend($user, EmailCodePurpose::ConfirmEmail);

        $message = $container->get('mailer.message_logger_listener')->getEvents()->getMessages()[0];
        self::assertInstanceOf(Email::class, $message);
        preg_match('/\d{6}/', (string) $message->getTextBody(), $matches);
        $code = $matches[0] ?? self::fail('No code in the email.');

        $issuer->verify($user, EmailCodePurpose::ConfirmEmail, $code);

        $this->expectException(AccountException::class);
        $issuer->verify($user, EmailCodePurpose::ConfirmEmail, $code);
    }

    public function testVerifyRejectsAWrongCodeForAnUnknownUser(): void
    {
        self::bootKernel();
        $container = self::getContainer();
        $user = $this->persistUser($container->get(EntityManagerInterface::class));

        $this->expectException(AccountException::class);
        $container->get(EmailCodeIssuer::class)->verify($user, EmailCodePurpose::ConfirmEmail, '000000');
    }

    private function persistUser(EntityManagerInterface $entityManager): User
    {
        $user = new User(\sprintf('issuer-%s@example.com', bin2hex(random_bytes(4))), 'Ada', 'en');
        $entityManager->persist($user);
        $entityManager->flush();

        return $user;
    }
}
