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
use App\Sync\Permission\CatalogLibraryView;
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
    ) {
    }

    /**
     * @return array{linked: list<string>, conflicts: list<string>} cloud library ids; both lists only name libraries the account can reach at the end
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

        return ['linked' => $linked, 'conflicts' => $conflicts];
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
                SELECT 1 FROM %s d WHERE d.cloudLibrary = l AND d.vouchedKey = :key AND d.publicKey = :key
                    AND LOWER(d.id) = :device AND d.member IS NOT NULL)
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
              WHERE d.cloudLibrary = :library AND m.cloudLibrary = :library AND d.vouchedKey = :key AND d.publicKey = :key AND LOWER(d.id) = :device',
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
        $taken = $this->entityManager->createQuery(\sprintf('SELECT 1 FROM %s m WHERE m.cloudLibrary = :library AND m.memberId = :member AND m.user <> :user', Membership::class))
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
        if (!$current instanceof Uuid) {
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
     * @return bool whether an operation was written
     */
    private function noteAccount(CloudLibrary $library, User $user, Uuid $person): bool
    {
        $cloud = $library->getCloudDeviceId();
        $named = new CatalogLibraryView($this->entityManager, $library)->cloudDevice();
        if (!$cloud instanceof Uuid || null === $named || strtolower($named) !== $cloud->toRfc4122()) {
            return false;
        }
        $account = $user->getAccountId()->toRfc4122();
        $row = $this->entityManager->createQuery(\sprintf('SELECT m.accountId AS account FROM %s m WHERE m.cloudLibrary = :library AND m.id = :member', Member::class))
            ->setParameter('library', $library->getId(), 'uuid')->setParameter('member', $person->toRfc4122())->setMaxResults(1)->getOneOrNullResult();
        // The person's row may not be in the catalog (deleted since): nothing to write on.
        if (!\is_array($row) || (\is_string($row['account']) && strtolower($row['account']) === $account)
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
}
