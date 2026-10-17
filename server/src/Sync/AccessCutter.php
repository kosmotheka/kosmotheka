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
use Symfony\Component\Clock\ClockInterface;
use Symfony\Component\Uid\Uuid;

/**
 * What the cloud stops giving (D98, D71), read from the catalog it keeps, never from a notice that could be lost or
 * from anything the client says (D84).
 *
 * Two cuts. **The device:** a revocation applied in the library names the authenticated device, and the catalog has
 * that id with the same key (the same device, not another one with its id). **The account:** its membership names a
 * person who now has the access withdrawn, or who no longer exists; the row is marked (`withdrawnAt`) and stops
 * giving access for good, even if the mark is taken off later: coming back is linking again (`AccountLinker`).
 */
final readonly class AccessCutter
{
    public function __construct(
        private EntityManagerInterface $entityManager,
        private CloudOperationRepository $operations,
        private CloudAuthor $author,
        private ClockInterface $clock,
    ) {
    }

    /** Whether the catalog says the person has no access: withdrawn, or not in the library any more. */
    public function personCut(CloudLibrary $library, Uuid $person): bool
    {
        $row = $this->entityManager->createQuery(\sprintf('SELECT m.accessWithdrawn AS withdrawn FROM %s m WHERE m.cloudLibrary = :library AND m.id = :member', Member::class))
            ->setParameter('library', $library->getId(), 'uuid')->setParameter('member', $person->toRfc4122())->setMaxResults(1)->getOneOrNullResult();

        return !\is_array($row) || true === $row['withdrawn'];
    }

    /** Whether a revocation applied in the library names this very device (id and key). */
    public function deviceCut(CloudLibrary $library, Device $device): bool
    {
        return [] !== $this->entityManager->createQuery(\sprintf(
            'SELECT 1 FROM %s r WHERE r.cloudLibrary = :library AND LOWER(r.deviceId) = :device
               AND EXISTS (SELECT 1 FROM %s d WHERE d.cloudLibrary = :library AND LOWER(d.id) = :device AND d.publicKey = :key)',
            Revocation::class,
            CatalogDevice::class,
        ))->setParameter('library', $library->getId(), 'uuid')->setParameter('device', strtolower($device->getDeviceId()))->setParameter('key', $device->getPublicKey())->setMaxResults(1)->getResult();
    }

    /**
     * The proofs a cut device gets: the signed revocations that name it; with a vector (a pull), also the operations
     * before them that it lacks from the same author, so the withdrawal chains with what it has. Nothing else, and
     * nothing at all to a device no revocation names: a cut account's other devices get the 403 without proofs.
     *
     * @param ?array<string, string> $vector
     *
     * @return list<array{schemaVersion: int, operation: \stdClass}>
     */
    public function proofs(CloudLibrary $library, ?Device $device, ?array $vector = null): array
    {
        // Only for the very device a revocation names, id and key (SEG-031): an account that was cut and signs in
        // with the id of somebody else's withdrawn device and a key of its own would otherwise read what the
        // author of that revocation wrote after the account lost the library.
        if (!$device instanceof Device || !$this->deviceCut($library, $device)) {
            return [];
        }

        return null === $vector
            ? $this->operations->revocationProofs($library, $device->getDeviceId())
            : $this->operations->revocationProofsWithChain($library, $device->getDeviceId(), $vector);
    }

    /**
     * Gives every row without a person the one its account's devices have (`fill`), then marks every membership of
     * the library whose person is cut, and, if the cloud has the power here, empties the
     * `account_id` of that person (`member.unlink_account`), once. Inside the library lock.
     *
     * @return list<User> the accounts that have just lost the library, to tell their devices
     */
    public function cut(CloudLibrary $library): array
    {
        return $this->entityManager->wrapInTransaction(function () use ($library): array {
            $this->entityManager->refresh($library, LockMode::PESSIMISTIC_WRITE);
            $rows = $this->entityManager->getRepository(Membership::class)->findBy(['cloudLibrary' => $library]);
            $cut = [];
            foreach ($rows as $row) {
                $this->fill($library, $row);
                $person = $row->getMemberId();
                if (!$row->isWithdrawn() && $person instanceof Uuid && $this->personCut($library, $person)) {
                    $row->withdraw($this->clock->now()->setTimezone(new \DateTimeZone('UTC')));
                    $cut[] = [$row->getUser(), $person];
                }
            }
            $this->entityManager->flush();
            if ([] === $cut) {
                return [];
            }
            foreach ($cut as [$user, $person]) {
                $this->unlinkAccount($library, $user, $person);
            }

            return array_map(static fn (array $pair): User => $pair[0], $cut);
        });
    }

    /**
     * Whether a row without a person gives no access (SEG-030): in a library with roles, the cloud is reached by
     * somebody, never by an account that is nobody there. Whoever uploaded starts without a person; the server
     * gives the row its person itself (`fill`), so nobody stays out of reach of a withdrawal by not asking.
     */
    public function needsAPerson(CloudLibrary $library): bool
    {
        return new CatalogLibraryView($this->entityManager, $library)->founded();
    }

    /**
     * Gives a row without a person the one its account's devices have in the catalog, without waiting for the app
     * to claim it: a device the account signed in with (id and key, in the server's own table) that the catalog has
     * with that same key and a person who has access. A vouched key, if there is one, must be that key; a row
     * without a person is only ever the uploader's, whose own sign-in proves the key. One person or none: two
     * devices of two persons fill nothing, and neither does a person another account already is.
     */
    private function fill(CloudLibrary $library, Membership $row): void
    {
        if ($row->isWithdrawn() || $row->getMemberId() instanceof Uuid) {
            return;
        }
        /** @var list<array{id: string}> $persons */
        $persons = $this->entityManager->createQuery(\sprintf(
            'SELECT DISTINCT m.id AS id FROM %s d JOIN d.member m, %s a
              WHERE a.user = :user AND d.cloudLibrary = :library AND m.cloudLibrary = :library
                AND LOWER(d.id) = LOWER(a.deviceId) AND d.publicKey = a.publicKey
                AND (d.vouchedKey IS NULL OR d.vouchedKey = \'\' OR d.vouchedKey = d.publicKey)
                AND (m.accessWithdrawn IS NULL OR m.accessWithdrawn = false)',
            CatalogDevice::class,
            Device::class,
        ))->setParameter('user', $row->getUser())->setParameter('library', $library->getId(), 'uuid')->setMaxResults(2)->getResult();
        if (1 !== \count($persons) || !Uuid::isValid($persons[0]['id'])) {
            return;
        }
        $person = Uuid::fromString($persons[0]['id']);
        $taken = $this->entityManager->createQuery(\sprintf('SELECT 1 FROM %s m WHERE m.cloudLibrary = :library AND m.memberId = :member AND m.user <> :user', Membership::class))
            ->setParameter('library', $library->getId(), 'uuid')->setParameter('member', $person, 'uuid')->setParameter('user', $row->getUser())->setMaxResults(1)->getResult();
        if ([] === $taken) {
            $row->linkTo($person);
        }
    }

    /** The account note goes: only if the cloud can write it here, the person is still in the catalog with that account, and no request of the same is in the log. */
    private function unlinkAccount(CloudLibrary $library, User $user, Uuid $person): void
    {
        $cloud = $this->author->power($library);
        if (!$cloud instanceof Uuid) {
            return;
        }
        $row = $this->entityManager->createQuery(\sprintf('SELECT m.accountId AS account FROM %s m WHERE m.cloudLibrary = :library AND m.id = :member', Member::class))
            ->setParameter('library', $library->getId(), 'uuid')->setParameter('member', $person->toRfc4122())->setMaxResults(1)->getOneOrNullResult();
        if (!\is_array($row) || !\is_string($row['account']) || strtolower($row['account']) !== $user->getAccountId()->toRfc4122()
            || $this->operations->hasRefusedAccountUnlink($library, $cloud->toRfc4122(), $person->toRfc4122())) {
            return;
        }
        try {
            $this->author->author($library, 'member.unlink_account', [['entity' => 'member', 'id' => $person->toRfc4122(), 'action' => 'update', 'fields' => ['account_id' => null]]]);
        } catch (CloudAuthorException) {
            // Without it the cut is the same: the row is marked.
        }
    }
}
