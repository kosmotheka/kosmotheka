<?php

declare(strict_types=1);

namespace App\Tests\Security;

use App\Entity\EmailCode;
use App\Entity\User;
use App\Enum\EmailCodePurpose;
use App\Security\AccountPurger;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;
use Symfony\Component\Clock\Test\ClockSensitiveTrait;

/** D49: old unconfirmed accounts and expired codes go, everything else stays. */
final class AccountPurgerTest extends KernelTestCase
{
    use ClockSensitiveTrait;

    public function testPurgeDeletesOnlyOldUnconfirmedUsersAndExpiredCodes(): void
    {
        self::mockTime('2026-01-02 12:00:00');
        self::bootKernel();
        $entityManager = self::getContainer()->get(EntityManagerInterface::class);

        $oldUnconfirmed = $this->persistUser($entityManager, 'old-unconfirmed', confirmed: false, createdAt: '2026-01-01 00:00:00');
        $freshUnconfirmed = $this->persistUser($entityManager, 'fresh-unconfirmed', confirmed: false, createdAt: '2026-01-02 11:00:00');
        $oldConfirmed = $this->persistUser($entityManager, 'old-confirmed', confirmed: true, createdAt: '2026-01-01 00:00:00');

        $expiredCode = $this->persistCode($entityManager, $oldConfirmed, '2026-01-01 00:00:00');
        $liveCode = $this->persistCode($entityManager, $freshUnconfirmed, '2026-01-03 00:00:00');

        $result = self::getContainer()->get(AccountPurger::class)->purge();

        self::assertSame(['unconfirmed_users' => 1, 'expired_codes' => 1, 'invitations' => 0, 'activity' => 0], $result);

        // The purger issues bulk DQL DELETEs, which bypass the identity map:
        // clear it so find() below actually hits the database.
        $entityManager->clear();

        $userRepository = $entityManager->getRepository(User::class);
        self::assertNull($userRepository->find($oldUnconfirmed->getId()));
        self::assertNotNull($userRepository->find($freshUnconfirmed->getId()));
        self::assertNotNull($userRepository->find($oldConfirmed->getId()));

        $codeRepository = $entityManager->getRepository(EmailCode::class);
        self::assertNull($codeRepository->find($expiredCode->getId()));
        self::assertNotNull($codeRepository->find($liveCode->getId()));
    }

    private function persistUser(EntityManagerInterface $entityManager, string $slug, bool $confirmed, string $createdAt): User
    {
        $user = new User(\sprintf('%s-%s@example.com', $slug, bin2hex(random_bytes(4))), 'Ada', 'en');
        if ($confirmed) {
            $user->confirm(new \DateTimeImmutable($createdAt));
        }
        new \ReflectionProperty($user, 'createdAt')->setValue($user, new \DateTimeImmutable($createdAt));

        $entityManager->persist($user);
        $entityManager->flush();

        return $user;
    }

    private function persistCode(EntityManagerInterface $entityManager, User $user, string $expiresAt): EmailCode
    {
        $code = new EmailCode($user, EmailCodePurpose::ResetPassword, hash('sha256', 'whatever'), new \DateTimeImmutable($expiresAt));
        $entityManager->persist($code);
        $entityManager->flush();

        return $code;
    }
}
