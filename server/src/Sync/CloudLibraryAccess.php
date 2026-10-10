<?php

declare(strict_types=1);

namespace App\Sync;

use App\Entity\CloudLibrary;
use App\Entity\Device;
use App\Entity\Membership;
use App\Entity\User;
use App\Repository\CloudLibraryRepository;
use App\Repository\MembershipRepository;
use App\Security\AccountException;
use Symfony\Component\Uid\Uuid;

/**
 * The door of every `{id}` cloud route (D84): the account must have a membership in that cloud library. An id that is
 * malformed, that does not exist or that belongs to somebody else gets the same answer (`404`), so nobody can learn
 * which ids exist. An account that **had** the library and was cut from it, or a device that was withdrawn from it
 * (D98, D71), gets `403 access_withdrawn` with the signed proofs: it knew the library, so nothing is revealed.
 */
final readonly class CloudLibraryAccess
{
    public function __construct(
        private CloudLibraryRepository $libraries,
        private MembershipRepository $memberships,
        private AccessCutter $cutter,
        private SyncNotifier $notifier,
    ) {
    }

    /**
     * @param ?array<string, string> $vector  the client's version vector, only on a pull: it decides how many earlier
     *                                        operations of the withdrawing author go with the proofs (never the access)
     * @param bool                   $toWrite a link or a push: an account that is nobody in a library with roles may
     *                                        still send, and gets nothing to read
     *
     * @throws AccountException not found, for any of the three first reasons above; `access_withdrawn` for a cut
     */
    public function require(string $id, User $user, ?Device $device, ?array $vector = null, bool $toWrite = false): CloudLibrary
    {
        if (!Uuid::isValid($id)) {
            throw AccountException::notFound();
        }
        $library = $this->libraries->find(Uuid::fromString($id)) ?? throw AccountException::notFound();
        $membership = $this->memberships->findOf($library, $user) ?? throw AccountException::notFound();
        if ($this->isCut($membership, $device, $toWrite)) {
            throw AccountException::accessWithdrawn($this->cutter->proofs($library, $device, $vector));
        }

        return $library;
    }

    /**
     * The libraries the account reaches from this device, and the ids of those it had and was cut from.
     *
     * @return array{libraries: list<CloudLibrary>, withdrawn: list<string>}
     */
    public function reachable(User $user, ?Device $device): array
    {
        $libraries = $withdrawn = [];
        foreach ($this->memberships->allOf($user) as $membership) {
            if ($this->isCut($membership, $device)) {
                $withdrawn[] = $membership->getCloudLibrary()->getId()->toRfc4122();
            } else {
                $libraries[] = $membership->getCloudLibrary();
            }
        }

        return ['libraries' => $libraries, 'withdrawn' => $withdrawn];
    }

    private function isCut(Membership $membership, ?Device $device, bool $toWrite = false): bool
    {
        if ($membership->isWithdrawn()) {
            return true;
        }
        $library = $membership->getCloudLibrary();
        $person = $membership->getMemberId();
        // A row without a person, in a library with roles (SEG-030): the server gives it its person now if the
        // account's devices have one; if they do not, the account is nobody there and the cloud gives it nothing.
        if (!$person instanceof Uuid && $this->cutter->needsAPerson($library)) {
            $this->cut($library);
            $person = $membership->getMemberId();
            // It may still send (`$toWrite`): what it sends is judged as nobody's, and an upload in several batches
            // has roles in the catalog before the batch that says whose the uploading device is.
            if (!$person instanceof Uuid) {
                return !$toWrite;
            }
        }
        // The catalog is read now, not a notice that could have been lost: the first request after the withdrawal marks the row.
        if ($person instanceof Uuid && $this->cutter->personCut($library, $person)) {
            $this->cut($library);

            return true;
        }

        return $device instanceof Device && $this->cutter->deviceCut($library, $device);
    }

    /** The cut, and the notices a push gives after one (D75): the cloud may have signed, and the accounts cut should hear. */
    private function cut(CloudLibrary $library): void
    {
        $cut = $this->cutter->cut($library);
        if ([] === $cut) {
            return;
        }
        $this->notifier->libraryChanged($library);
        foreach ($cut as $user) {
            $this->notifier->librariesOfAccountChanged($user);
        }
    }
}
