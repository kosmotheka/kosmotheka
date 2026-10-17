<?php

declare(strict_types=1);

namespace App\Security;

use App\Entity\User;
use App\Enum\ActivityKind;
use App\Sync\CloudOwnership;
use Doctrine\ORM\EntityManagerInterface;

/**
 * Deleting an account, the one way: by the route (`DELETE /v1/account`) and by the console (`app:user:delete`). One
 * transaction: the libraries the account uploaded pass to someone else with a place in them, or go with it if there is
 * nobody (D99), and then the account goes. The line of the log keeps the account's id and the fingerprint of its email,
 * not the email: it outlives the account (D66).
 */
final readonly class AccountDeletion
{
    public function __construct(
        private EntityManagerInterface $entityManager,
        private CloudOwnership $cloudOwnership,
        private ActivityLog $activityLog,
    ) {
    }

    /** @param ?string $deviceId the device that asked; null from the console, which has none */
    public function delete(User $user, ?string $deviceId): void
    {
        $this->entityManager->wrapInTransaction(function () use ($user, $deviceId): void {
            $this->cloudOwnership->handOverLibrariesOf($user);
            $this->activityLog->record(ActivityKind::AccountDeleted, $user, $deviceId);
            $this->entityManager->remove($user);
        });
    }
}
