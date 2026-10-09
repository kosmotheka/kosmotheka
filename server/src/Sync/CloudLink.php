<?php

declare(strict_types=1);

namespace App\Sync;

use App\Dto\OperationPayload;
use App\Dto\PushedOperation;
use App\Entity\CloudLibrary;
use App\Entity\Device;
use App\Enum\CatalogEntity;
use App\Enum\ChangeAction;
use App\Enum\OperationRejection;
use App\Repository\DeviceRepository;
use App\Security\AccountException;
use Doctrine\DBAL\LockMode;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Uid\Uuid;

/**
 * Links a cloud library to the local library it holds (D84, D93): the link operation names this cloud
 * library, and is signed by a device of the account that owns it. Two ways in, one set of checks: the
 * link route, which links without storing the operation, and CloudPush, when the link comes in a batch
 * (apps from before the route).
 */
final readonly class CloudLink
{
    public function __construct(
        private EntityManagerInterface $entityManager,
        private OperationReader $reader,
        private OperationSignature $signature,
        private DeviceRepository $devices,
        private SchemaVersion $schemaVersion,
    ) {
    }

    /**
     * The link route: checks the operation as a push would and links the library, without storing it (it comes
     * with the log, in its place). A library already linked to this local id answers as the first time; to another, no.
     *
     * @throws AccountException `link_rejected` for anything the checks refuse
     */
    public function linkFrom(CloudLibrary $library, PushedOperation $pushed): void
    {
        $this->entityManager->wrapInTransaction(function () use ($library, $pushed): void {
            // The same lock as a push, so a push and a link never decide on the same state.
            $this->entityManager->refresh($library, LockMode::PESSIMISTIC_WRITE);
            try {
                if ($pushed->schemaVersion > $this->schemaVersion->current()) {
                    throw new OperationRejectedException(OperationRejection::NewerVersion);
                }
                $payload = $this->reader->read($pushed->operation);
                $this->check($library, $pushed, $payload);
                if ($library->isLinked()) {
                    if (strtolower($payload->library) !== $library->getLocalLibraryId()) {
                        throw new OperationRejectedException(OperationRejection::OtherLibrary);
                    }
                    // A library linked before the link was remembered (7.4.6) learns it here.
                    $this->note($library, $payload);

                    return;
                }
                $this->bind($library, $payload);
            } catch (OperationRejectedException) {
                throw AccountException::linkRejected();
            }
        });
    }

    /**
     * Links the library with a link operation the push carries.
     *
     * @throws OperationRejectedException unless this is the library's own link, signed by a device of its owner
     */
    public function link(CloudLibrary $library, PushedOperation $pushed, OperationPayload $payload): void
    {
        $this->check($library, $pushed, $payload);
        $this->bind($library, $payload);
    }

    /**
     * Whether this is the library's link, signed by a device of its owner: what an operation must be to be stored
     * under the id remembered as the link's. A device removed since then still counts; it signed while it could.
     */
    public function holds(CloudLibrary $library, PushedOperation $pushed, OperationPayload $payload): bool
    {
        try {
            $this->signedByOwner($library, $pushed, $payload);

            return true;
        } catch (OperationRejectedException) {
            return false;
        }
    }

    /** Remembers the link of a library that has none remembered; the caller has checked the operation. */
    public function note(CloudLibrary $library, OperationPayload $payload): void
    {
        $library->noteLinkOperation(Uuid::fromString($payload->id));
        $this->entityManager->flush();
    }

    /** Links the library and remembers which operation did: the one whose signature was just checked (D93), whenever it is stored. */
    private function bind(CloudLibrary $library, OperationPayload $payload): void
    {
        $library->link($payload->library);
        $this->note($library, $payload);
    }

    public function isLink(CloudLibrary $library, OperationPayload $payload): bool
    {
        foreach ($payload->changes as $change) {
            $cloudId = $change->fields['cloud_library_id'] ?? null;
            if (CatalogEntity::Library === $change->entity && ChangeAction::Update === $change->action
                && strtolower($change->id) === strtolower($payload->library)
                && \is_string($cloudId) && strtolower($cloudId) === $library->getId()->toRfc4122()) {
                return true;
            }
        }

        return false;
    }

    /**
     * @throws OperationRejectedException
     */
    private function check(CloudLibrary $library, PushedOperation $pushed, OperationPayload $payload): void
    {
        if ($this->signedByOwner($library, $pushed, $payload)->isRemoved()) {
            throw new OperationRejectedException(OperationRejection::UnknownKey);
        }
    }

    /**
     * @throws OperationRejectedException
     */
    private function signedByOwner(CloudLibrary $library, PushedOperation $pushed, OperationPayload $payload): Device
    {
        if (!$this->isLink($library, $payload)) {
            throw new OperationRejectedException(OperationRejection::NotLinked);
        }

        // The owner's own account devices, found by the account and the id from the HLC, never by the id alone.
        $device = $this->devices->findOneOfUser($library->getOwner(), explode('-', $payload->hlc, 3)[2]);
        if (!$device instanceof Device) {
            throw new OperationRejectedException(OperationRejection::UnknownKey);
        }
        try {
            $this->signature->verify($pushed->operation, $device->getPublicKey());
        } catch (InvalidSignatureException) {
            throw new OperationRejectedException(OperationRejection::BadSignature);
        }

        return $device;
    }
}
