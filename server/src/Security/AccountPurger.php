<?php

declare(strict_types=1);

namespace App\Security;

use App\Entity\EmailCode;
use App\Entity\User;
use App\Repository\ActivityEntryRepository;
use App\Repository\GoogleAttemptRepository;
use App\Repository\InvitationRepository;
use App\Repository\MetadataCacheRepository;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;

/**
 * Deletes what D49 says must not linger: accounts never confirmed after 24h,
 * invitations a week after they expire (D43), activity-log lines after a year (D66), and email codes past their expiry
 * (confirmed accounts can also leave dead codes behind, e.g. an unused reset_password code). It also empties the
 * metadata cache of books past their age (7.8). Run hourly by
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
        private MetadataCacheRepository $metadataCache,
    ) {
    }

    /** Invitations are kept this long after they expire, so the administrator's list still shows what became of them. */
    private const int INVITATION_RETENTION_DAYS = 7;

    /** How long the activity log keeps a line (D66, accepted on 01/10/2026): a year, then it is deleted. The only place the period is written. */
    public const string ACTIVITY_RETENTION = '-1 year';

    /**
     * Each step on its own, so that one that fails does not keep the others waiting for the next hour (CAL-008).
     *
     * @return array{unconfirmed_users: int, expired_codes: int, invitations: int, activity: int, google_attempts: int, metadata_cache: int}
     *
     * @throws PurgeFailedException after running every step, if any failed
     */
    public function purge(): array
    {
        $now = $this->clock->now()->setTimezone(new \DateTimeZone('UTC'));
        $steps = [
            'unconfirmed_users' => fn (): int => $this->entityManager->createQuery(
                \sprintf('DELETE FROM %s u WHERE u.confirmedAt IS NULL AND u.createdAt < :cutoff', User::class)
            )->setParameter('cutoff', $now->modify(\sprintf('-%d hours', self::UNCONFIRMED_TTL_HOURS)))->execute(),
            'expired_codes' => fn (): int => $this->entityManager->createQuery(
                \sprintf('DELETE FROM %s c WHERE c.expiresAt < :now', EmailCode::class)
            )->setParameter('now', $now)->execute(),
            'invitations' => fn (): int => $this->invitations->deleteExpiredBefore($now->modify(\sprintf('-%d days', self::INVITATION_RETENTION_DAYS))),
            'activity' => fn (): int => $this->activity->deleteBefore($now->modify(self::ACTIVITY_RETENTION)),
            'google_attempts' => fn (): int => $this->googleAttempts->deleteExpiredBefore($now),
            'metadata_cache' => fn (): int => $this->metadataCache->deleteStaleBefore($now),
        ];

        $deleted = [];
        $failed = [];
        $firstFailure = null;
        foreach ($steps as $name => $step) {
            try {
                // A transaction of its own (a savepoint inside another one): a failed statement does not spoil the next.
                $deleted[$name] = $this->entityManager->getConnection()->transactional(static fn (): int => $step());
            } catch (\Throwable $failure) {
                $deleted[$name] = 0;
                $failed[] = $name;
                $firstFailure ??= $failure;
            }
        }
        if ($firstFailure instanceof \Throwable) {
            throw new PurgeFailedException($failed, $firstFailure);
        }

        return $deleted;
    }
}
