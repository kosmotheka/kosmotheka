<?php

declare(strict_types=1);

namespace App\Tests\Security;

use App\Entity\EmailCode;
use App\Entity\MetadataCacheEntry;
use App\Entity\User;
use App\Enum\EmailCodePurpose;
use App\Security\AccountPurger;
use App\Security\PurgeFailedException;
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

        self::assertSame(['unconfirmed_users' => 1, 'expired_codes' => 1, 'invitations' => 0, 'activity' => 0, 'google_attempts' => 0, 'metadata_cache' => 0, 'covers' => 0], $result);

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

    public function testAStepThatFailsDoesNotKeepTheNextOnesFromRunning(): void
    {
        self::mockTime('2026-06-30 12:00:00');
        self::bootKernel();
        $entityManager = self::getContainer()->get(EntityManagerInterface::class);
        $entityManager->persist(new MetadataCacheEntry('google_books', '9780000000002', true, [], new \DateTimeImmutable('2026-05-01 00:00:00')));
        $entityManager->flush();
        // Undone with the test's transaction: the step for invitations fails, as with a table locked or broken.
        $entityManager->getConnection()->executeStatement('ALTER TABLE invitation RENAME TO invitation_away');

        try {
            self::getContainer()->get(AccountPurger::class)->purge();
            self::fail('The failed step must still be told.');
        } catch (PurgeFailedException $exception) {
            self::assertSame(['invitations'], $exception->steps);
        }

        $entityManager->clear();
        self::assertSame([], $entityManager->getRepository(MetadataCacheEntry::class)->findBy(['code' => '9780000000002']));
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
