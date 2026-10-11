<?php

declare(strict_types=1);

namespace App\Security;

use App\Entity\EmailCode;
use App\Entity\User;
use App\Repository\ActivityEntryRepository;
use App\Repository\GoogleAttemptRepository;
use App\Repository\InvitationRepository;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;

/**
 * Deletes what D49 says must not linger: accounts never confirmed after 24h,
 * invitations a week after they expire (D43), activity-log lines after a year (D66), and email codes past their expiry (confirmed accounts can also leave dead
 * codes behind, e.g. an unused reset_password code). Run hourly by
 * App\Scheduler\PurgeAccountsMessageHandler, and by hand with
 * `app:accounts:purge`.
 *
 * Deleting a user cascades to its devices and email codes at the database
 * level (ON DELETE CASCADE), so the DQL deletes below never orphan a row.
 */
final readonly class AccountPurger
{
    private const int UNCONFIRMED_TTL_HOURS = 24;

    public function __construct(
        private EntityManagerInterface $entityManager,
        private ClockInterface $clock,
        private InvitationRepository $invitations,
        private ActivityEntryRepository $activity,
        private GoogleAttemptRepository $googleAttempts,
    ) {
    }

    /** Invitations are kept this long after they expire, so the administrator's list still shows what became of them. */
    private const int INVITATION_RETENTION_DAYS = 7;

    /** How long the activity log keeps a line (D66, accepted on 01/10/2026): a year, then it is deleted. The only place the period is written. */
    public const string ACTIVITY_RETENTION = '-1 year';

    /** @return array{unconfirmed_users: int, expired_codes: int, invitations: int, activity: int, google_attempts: int} */
    public function purge(): array
    {
        $now = $this->clock->now()->setTimezone(new \DateTimeZone('UTC'));
        $unconfirmedCutoff = $now->modify(\sprintf('-%d hours', self::UNCONFIRMED_TTL_HOURS));

        $deletedUsers = $this->entityManager->createQuery(
            \sprintf('DELETE FROM %s u WHERE u.confirmedAt IS NULL AND u.createdAt < :cutoff', User::class)
        )->setParameter('cutoff', $unconfirmedCutoff)->execute();

        $deletedCodes = $this->entityManager->createQuery(
            \sprintf('DELETE FROM %s c WHERE c.expiresAt < :now', EmailCode::class)
        )->setParameter('now', $now)->execute();

        $deletedInvitations = $this->invitations->deleteExpiredBefore($now->modify(\sprintf('-%d days', self::INVITATION_RETENTION_DAYS)));

        $deletedActivity = $this->activity->deleteBefore($now->modify(self::ACTIVITY_RETENTION));

        $deletedAttempts = $this->googleAttempts->deleteExpiredBefore($now);

        return ['unconfirmed_users' => $deletedUsers, 'expired_codes' => $deletedCodes, 'invitations' => $deletedInvitations, 'activity' => $deletedActivity, 'google_attempts' => $deletedAttempts];
    }
}
