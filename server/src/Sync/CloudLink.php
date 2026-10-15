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
use App\Repository\CloudDeviceKeyRepository;
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
 *
 * The owner counts only for the first link (D93, D99). The key that signed it is remembered with the link
 * (`link_signer_key`), and the same operation is recognised by that key afterwards, whoever uploads it and
 * whatever account owns the library by then: the owner can change when the account that uploaded it is deleted.
 */
final readonly class CloudLink
{
    public function __construct(
        private EntityManagerInterface $entityManager,
        private OperationReader $reader,
        private OperationSignature $signature,
        private DeviceRepository $devices,
        private SchemaVersion $schemaVersion,
        private CloudDeviceKeyRepository $deviceKeys,
        private CloudIdentityIssuer $identity,
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
            // A library from before the server had a secret gets its cloud device when it is linked, if not before.
            $this->identity->ensure($library);
            try {
                if ($pushed->schemaVersion > $this->schemaVersion->current()) {
                    throw new OperationRejectedException(OperationRejection::NewerVersion);
                }
                $payload = $this->reader->read($pushed->operation);
                $signer = $this->check($library, $pushed, $payload);
                if ($library->isLinked()) {
                    if (strtolower($payload->library) !== $library->getLocalLibraryId()) {
                        throw new OperationRejectedException(OperationRejection::OtherLibrary);
                    }
                    // A library linked before the link was remembered (7.4.6) learns it here.
                    $this->note($library, $payload, $signer);

                    return;
                }
                $this->bind($library, $payload, $signer);
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
        $this->bind($library, $payload, $this->check($library, $pushed, $payload));
    }

    /**
     * Whether this is the library's link: what an operation must be to be stored under the id remembered as the
     * link's. It names this cloud library and is signed with the key that signed the link, remembered with it; a
     * device removed since then, or an owner changed since then, does not matter (D99). A library linked before the
     * key was remembered is judged as the first link was, by a device of its owner, and learns the key if it holds.
     */
    public function holds(CloudLibrary $library, PushedOperation $pushed, OperationPayload $payload): bool
    {
        try {
            if (null !== $library->getLinkSignerKey()) {
                $this->signedByLinkSigner($library, $pushed, $payload);

                return true;
            }
            $device = $this->signedByOwner($library, $pushed, $payload);
        } catch (OperationRejectedException) {
            return false;
        }
        $library->noteLinkSigner($device->getPublicKey());
        $this->entityManager->flush();

        return true;
    }

    /**
     * Remembers the link of a library that has none remembered, and the key that signed it; the caller has checked the
     * operation.
     */
    public function note(CloudLibrary $library, OperationPayload $payload, ?string $signerKey = null): void
    {
        $library->noteLinkOperation(Uuid::fromString($payload->id));
        if (null !== $signerKey) {
            $library->noteLinkSigner($signerKey);
        }
        $this->entityManager->flush();
    }

    /** Links the library and remembers which operation did, and which key signed it: the ones just checked (D93), whenever it is stored. */
    private function bind(CloudLibrary $library, OperationPayload $payload, string $signerKey): void
    {
        $library->link($payload->library);
        $this->note($library, $payload, $signerKey);
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
     * @return string the public key that signed the link
     *
     * @throws OperationRejectedException
     */
    private function check(CloudLibrary $library, PushedOperation $pushed, OperationPayload $payload): string
    {
        // The link may name the cloud's device (D95), and it can only name it with the key the server registered.
        $this->refuseKeyChanges($library, $payload);
        // Linked already, with the signer remembered: the same signer again, whoever owns the library now (D99).
        if ($library->isLinked() && null !== $library->getLinkSignerKey()) {
            return $this->signedByLinkSigner($library, $pushed, $payload);
        }
        $device = $this->signedByOwner($library, $pushed, $payload);
        if ($device->isRemoved()) {
            throw new OperationRejectedException(OperationRejection::UnknownKey);
        }

        return $device->getPublicKey();
    }

    /**
     * The link as it is recognised once it is linked: it names this cloud library and carries a good signature of the
     * key that signed the link. Never the owner's devices (D99).
     *
     * @return string that key
     *
     * @throws OperationRejectedException
     */
    private function signedByLinkSigner(CloudLibrary $library, PushedOperation $pushed, OperationPayload $payload): string
    {
        $key = $library->getLinkSignerKey();
        if (null === $key || !$this->isLink($library, $payload)) {
            throw new OperationRejectedException(OperationRejection::NotLinked);
        }
        try {
            $this->signature->verify($pushed->operation, $key);
        } catch (InvalidSignatureException) {
            throw new OperationRejectedException(OperationRejection::BadSignature);
        }

        return $key;
    }

    /**
     * A `device` change that offers another key than the one registered for that id in this library is refused,
     * whoever signs it (as in the app's signature_holds): a device's key never changes, and the cloud's is
     * registered from the day the library exists.
     *
     * @throws OperationRejectedException with `KeyChanged`
     */
    public function refuseKeyChanges(CloudLibrary $library, OperationPayload $payload): void
    {
        foreach ($payload->changes as $change) {
            $offered = $change->fields['public_key'] ?? null;
            if (CatalogEntity::Device !== $change->entity || !\is_string($offered)) {
                continue;
            }
            $registered = $this->deviceKeys->findOneInLibrary($library, $change->id)?->getPublicKey();
            if (null !== $registered && !hash_equals($registered, $offered)) {
                throw new OperationRejectedException(OperationRejection::KeyChanged);
            }
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
