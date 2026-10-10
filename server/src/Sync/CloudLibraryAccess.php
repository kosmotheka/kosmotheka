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
    ) {
    }

    /**
     * @param ?array<string, string> $vector the client's version vector, only on a pull: it decides how many earlier
     *                                       operations of the withdrawing author go with the proofs (never the access)
     *
     * @throws AccountException not found, for any of the three first reasons above; `access_withdrawn` for a cut
     */
    public function require(string $id, User $user, ?Device $device, ?array $vector = null): CloudLibrary
    {
        if (!Uuid::isValid($id)) {
            throw AccountException::notFound();
        }
        $library = $this->libraries->find(Uuid::fromString($id)) ?? throw AccountException::notFound();
        $membership = $this->memberships->findOf($library, $user) ?? throw AccountException::notFound();
        if ($this->isCut($membership, $device)) {
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

    private function isCut(Membership $membership, ?Device $device): bool
    {
        if ($membership->isWithdrawn()) {
            return true;
        }
        $library = $membership->getCloudLibrary();
        $person = $membership->getMemberId();
        // The catalog is read now, not a notice that could have been lost: the first request after the withdrawal marks the row.
        if ($person instanceof Uuid && $this->cutter->personCut($library, $person)) {
            $this->cutter->cut($library);

            return true;
        }

        return $device instanceof Device && $this->cutter->deviceCut($library, $device);
    }
}
