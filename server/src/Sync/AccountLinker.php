<?php

declare(strict_types=1);

namespace App\Sync;

use App\Entity\Catalog\Device as CatalogDevice;
use App\Entity\Catalog\Member;
use App\Entity\Catalog\Revocation;
use App\Entity\CloudLibrary;
use App\Entity\Device;
use App\Entity\Membership;
use App\Entity\User;
use App\Repository\CloudOperationRepository;
use Doctrine\DBAL\LockMode;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Uid\Uuid;

/**
 * Links an account with its person in every cloud library where the device it signed in with is already that
 * person's, with a key somebody vouched for (D76, D96). The link is a `membership` row, which is what gives access.
 *
 * Nothing the client says decides anything (D84): the device is the one the request was authenticated with (its
 * id and key, proved with the signed challenge of 7.1), and the rest is read from the server's own tables.
 * A device counts only if the catalog has it with the same key as `public_key` **and** as `vouched_key` and with
 * a person: an assignment without a vouched key, or with another one, links nobody, even if the judge still
 * believes it for the role. Access gives no role: the person's role is the library's.
 */
final readonly class AccountLinker
{
    public function __construct(
        private EntityManagerInterface $entityManager,
        private CloudOperationRepository $operations,
        private CloudAuthor $author,
        private SyncNotifier $notifier,
        private AccessCutter $cutter,
    ) {
    }

    /**
     * @return array{linked: list<string>, conflicts: list<string>, devices: list<string>} cloud library ids; the lists only name libraries the account can reach at the end
     */
    public function claim(Device $device): array
    {
        $user = $device->getUser();
        $linked = $conflicts = [];
        foreach ($this->candidates($device) as $library) {
            $written = false;
            $outcome = $this->entityManager->wrapInTransaction(function () use ($library, $device, $user, &$written): ?string {
                // The lock of every push: two claims, or a claim and a push, take turns, and what follows is read inside it.
                $this->entityManager->refresh($library, LockMode::PESSIMISTIC_WRITE);
                $person = $this->personOf($library, $device);
                if (!$person instanceof Uuid || $this->revoked($library, $device)) {
                    return null;
                }
                $outcome = $this->link($library, $user, $person);
                if (null !== $outcome && 'conflict' !== $outcome) {
                    $written = $this->noteAccount($library, $user, $person);
                }

                return $outcome;
            });
            if ($written) {
                $this->notifier->libraryChanged($library);
            }
            if ('new' === $outcome) {
                $linked[] = $library->getId()->toRfc4122();
            } elseif ('conflict' === $outcome) {
                $conflicts[] = $library->getId()->toRfc4122();
            }
        }

        return ['linked' => $linked, 'conflicts' => $conflicts, 'devices' => $this->linkDevice($device)];
    }

    /**
     * The second step: where the account is already a person, the cloud says this device is that person's, vouching
     * for the key it signed in with (D76, D96). One query finds the libraries to look at: those where the account has
     * a membership with a person. Each is judged again inside the library lock.
     *
     * @return list<string> cloud library ids where the device has just been linked
     */
    private function linkDevice(Device $device): array
    {
        /** @var list<CloudLibrary> $libraries */
        $libraries = $this->entityManager->createQuery(\sprintf(
            'SELECT l FROM %s l JOIN %s m WITH m.cloudLibrary = l
              WHERE m.user = :user AND m.memberId IS NOT NULL AND m.withdrawnAt IS NULL AND l.localLibraryId IS NOT NULL ORDER BY l.createdAt',
            CloudLibrary::class,
            Membership::class,
        ))->setParameter('user', $device->getUser())->getResult();
        $linked = [];
        foreach ($libraries as $library) {
            $wrote = $done = false;
            $this->entityManager->wrapInTransaction(function () use ($library, $device, &$wrote, &$done): void {
                $this->entityManager->refresh($library, LockMode::PESSIMISTIC_WRITE);
                $done = $this->linkDeviceIn($library, $device, $wrote);
            });
            if ($wrote || $done) {
                $this->notifier->libraryChanged($library);
            }
            if ($done) {
                $linked[] = $library->getId()->toRfc4122();
            }
        }

        return $linked;
    }

    /**
     * @param bool $wrote set when the cloud wrote anything here, the account note included, so the clients are told
     *
     * @return bool whether the device was linked
     */
    private function linkDeviceIn(CloudLibrary $library, Device $device, bool &$wrote): bool
    {
        $user = $device->getUser();
        $membership = $this->entityManager->getRepository(Membership::class)->findOneBy(['cloudLibrary' => $library, 'user' => $user]);
        if (!$membership instanceof Membership) {
            return false;
        }
        $this->entityManager->refresh($membership);
        $person = $membership->getMemberId();
        $cloud = $this->author->power($library);
        if (!$person instanceof Uuid || !$cloud instanceof Uuid || $this->cutter->personCut($library, $person)) {
            return false;
        }
        // The regla 12 asks that the person already has this account: write it first, in this same call.
        $account = $user->getAccountId()->toRfc4122();
        if ($this->accountOf($library, $person) !== $account) {
            $wrote = $this->noteAccount($library, $user, $person);
            if ($this->accountOf($library, $person) !== $account) {
                return false;
            }
        }
        // The device's own row, registered by the device itself with this very key, and still nobody's.
        $row = $this->entityManager->createQuery(\sprintf(
            'SELECT d.id AS id FROM %s d WHERE d.cloudLibrary = :library AND LOWER(d.id) = :device AND d.publicKey = :key
                AND d.member IS NULL AND d.trustedThrough IS NULL',
            CatalogDevice::class,
        ))->setParameter('library', $library->getId(), 'uuid')->setParameter('device', strtolower($device->getDeviceId()))->setParameter('key', $device->getPublicKey())->setMaxResults(1)->getOneOrNullResult();
        $id = \is_array($row) ? $row['id'] : null;
        if (!\is_string($id) || $this->revoked($library, $device)
            || $this->operations->hasRefusedDeviceLink($library, $cloud->toRfc4122(), $id, $person->toRfc4122(), $device->getPublicKey())) {
            return false;
        }
        try {
            $this->author->author($library, 'device.link_account', [[
                'entity' => 'device',
                'id' => $id,
                'action' => 'update',
                'fields' => ['member_id' => $person->toRfc4122(), 'vouched_key' => $device->getPublicKey()],
            ]]);
        } catch (CloudAuthorException) {
            return false;
        }
        $wrote = true;

        return true;
    }

    /**
     * The linked cloud libraries whose catalog has this device with this key as its registered and as its vouched
     * key, and a person: one query, whatever number of libraries the server holds (`idx_catalog_devices_vouched_key`).
     *
     * @return list<CloudLibrary>
     */
    private function candidates(Device $device): array
    {
        /** @var list<CloudLibrary> $libraries */
        $libraries = $this->entityManager->createQuery(\sprintf(
            'SELECT l FROM %s l WHERE l.localLibraryId IS NOT NULL AND EXISTS (
                SELECT 1 FROM %s d JOIN d.member pm WHERE d.cloudLibrary = l AND d.vouchedKey = :key AND d.publicKey = :key
                    AND LOWER(d.id) = :device AND (pm.accessWithdrawn IS NULL OR pm.accessWithdrawn = false))
             ORDER BY l.createdAt',
            CloudLibrary::class,
            CatalogDevice::class,
        ))->setParameter('key', $device->getPublicKey())->setParameter('device', strtolower($device->getDeviceId()))->getResult();

        return $libraries;
    }

    /** The person (member id) of the device, read again: same conditions as the candidate query. */
    private function personOf(CloudLibrary $library, Device $device): ?Uuid
    {
        $row = $this->entityManager->createQuery(\sprintf(
            'SELECT m.id AS id FROM %s d JOIN d.member m
              WHERE d.cloudLibrary = :library AND m.cloudLibrary = :library AND d.vouchedKey = :key AND d.publicKey = :key AND LOWER(d.id) = :device
                AND (m.accessWithdrawn IS NULL OR m.accessWithdrawn = false)',
            CatalogDevice::class,
        ))->setParameter('library', $library->getId(), 'uuid')->setParameter('key', $device->getPublicKey())->setParameter('device', strtolower($device->getDeviceId()))->setMaxResults(1)->getOneOrNullResult();
        $id = \is_array($row) ? $row['id'] : null;

        return \is_string($id) && Uuid::isValid($id) ? Uuid::fromString($id) : null;
    }

    private function revoked(CloudLibrary $library, Device $device): bool
    {
        return [] !== $this->entityManager->createQuery(\sprintf('SELECT 1 FROM %s r WHERE r.cloudLibrary = :library AND LOWER(r.deviceId) = :device', Revocation::class))
            ->setParameter('library', $library->getId(), 'uuid')->setParameter('device', strtolower($device->getDeviceId()))->setMaxResults(1)->getResult();
    }

    /**
     * @return ?string `new` (the account gained access or was given its person), `same` (already linked), `conflict`, or null (another account is that person's)
     */
    private function link(CloudLibrary $library, User $user, Uuid $person): ?string
    {
        // One person, one account.
        $taken = $this->entityManager->createQuery(\sprintf('SELECT 1 FROM %s m WHERE m.cloudLibrary = :library AND m.memberId = :member AND m.user <> :user AND m.withdrawnAt IS NULL', Membership::class))
            ->setParameter('library', $library->getId(), 'uuid')->setParameter('member', $person, 'uuid')->setParameter('user', $user)->setMaxResults(1)->getResult();
        if ([] !== $taken) {
            return null;
        }
        $membership = $this->entityManager->getRepository(Membership::class)->findOneBy(['cloudLibrary' => $library, 'user' => $user]);
        if (!$membership instanceof Membership) {
            $this->entityManager->persist(new Membership($user, $library, $person));
            $this->entityManager->flush();

            return 'new';
        }
        $this->entityManager->refresh($membership);
        $current = $membership->getMemberId();
        // Coming back after a cut is linking again, the usual way (D98).
        if (!$current instanceof Uuid || $membership->isWithdrawn()) {
            $membership->linkTo($person);
            $this->entityManager->flush();

            return 'new';
        }

        return $current->equals($person) ? 'same' : 'conflict';
    }

    /**
     * Writes `member.account_id` as the cloud, but only if it has the power in this library now (it is the device
     * the catalog names, and it is registered with the key its id derives from): without it the operation would
     * only be a request. And not again when a request of the same link is already in the log.
     *
     * Also used by an accepted invitation (`InvitationAcceptor`), inside the library lock and a transaction.
     *
     * @return bool whether an operation was written
     */
    public function noteAccount(CloudLibrary $library, User $user, Uuid $person): bool
    {
        $cloud = $this->author->power($library);
        if (!$cloud instanceof Uuid) {
            return false;
        }
        $account = $user->getAccountId()->toRfc4122();
        $current = $this->accountOf($library, $person);
        // The person's row may not be in the catalog (deleted since): nothing to write on.
        if (false === $current || $current === $account
            || $this->operations->hasRefusedAccountLink($library, $cloud->toRfc4122(), $person->toRfc4122(), $account)) {
            return false;
        }
        try {
            $this->author->author($library, 'member.link_account', [[
                'entity' => 'member',
                'id' => $person->toRfc4122(),
                'action' => 'update',
                'fields' => ['account_id' => $account],
            ]]);
        } catch (CloudAuthorException) {
            // The cloud cannot write here (no secret, an operation the log refuses): access is given all the same.
            return false;
        }

        return true;
    }

    /**
     * The `account_id` of a person in the catalog, lower-cased; null if the person has none; false if there is no such person.
     */
    private function accountOf(CloudLibrary $library, Uuid $person): string|false|null
    {
        $row = $this->entityManager->createQuery(\sprintf('SELECT m.accountId AS account FROM %s m WHERE m.cloudLibrary = :library AND m.id = :member', Member::class))
            ->setParameter('library', $library->getId(), 'uuid')->setParameter('member', $person->toRfc4122())->setMaxResults(1)->getOneOrNullResult();
        if (!\is_array($row)) {
            return false;
        }

        return \is_string($row['account']) && '' !== $row['account'] ? strtolower($row['account']) : null;
    }
}
