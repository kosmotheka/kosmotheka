<?php

declare(strict_types=1);

namespace App\Sync;

use App\Dto\OperationPayload;
use App\Dto\PushedOperation;
use App\Dto\PushRequest;
use App\Dto\PushResult;
use App\Entity\CloudDeviceKey;
use App\Entity\CloudLibrary;
use App\Entity\Device;
use App\Enum\CatalogEntity;
use App\Enum\ChangeAction;
use App\Enum\OperationRejection;
use App\Repository\CloudDeviceKeyRepository;
use App\Repository\CloudOperationRepository;
use App\Repository\DeviceRepository;
use App\Security\AccountException;
use Doctrine\DBAL\LockMode;
use Doctrine\ORM\EntityManagerInterface;

/**
 * The rules for what a cloud library accepts (D84; docs/architecture/cloud.md).
 * It judges each operation only by what it says (structure, library, signature),
 * never by when it arrives (seam 7), and it neither reads ids from the client to
 * decide access nor judges roles (milestone 7.4). The caller has already checked
 * access (App\Sync\CloudLibraryAccess).
 *
 * Until the library is linked, only the link operation may enter (seam 3):
 * a change of the `library` row naming this library's cloud id, signed by a
 * device of the account that owns it.
 */
final readonly class CloudPush
{
    public function __construct(
        private EntityManagerInterface $entityManager,
        private SchemaVersion $schemaVersion,
        private OperationReader $reader,
        private OperationSignature $signature,
        private CloudOperationRepository $operations,
        private CloudDeviceKeyRepository $deviceKeys,
        private DeviceRepository $devices,
    ) {
    }

    /**
     * @throws AccountException when an operation comes from another schema version
     */
    public function push(CloudLibrary $library, PushRequest $request): PushResult
    {
        // A client on another version does not sync at all (D31), so this is not a per-operation rejection.
        foreach ($request->operations as $pushed) {
            $this->schemaVersion->require($pushed->schemaVersion);
        }

        return $this->entityManager->wrapInTransaction(fn (): PushResult => $this->apply($library, $request));
    }

    private function apply(CloudLibrary $library, PushRequest $request): PushResult
    {
        // One push at a time per library: linking and registering a key check first and write after.
        // The refresh also brings the link state as it is now, not as it was when the library was loaded.
        $this->entityManager->refresh($library, LockMode::PESSIMISTIC_WRITE);

        $received = $known = $rejected = 0;
        $readable = [];
        foreach ($request->operations as $pushed) {
            try {
                $readable[] = ['pushed' => $pushed, 'payload' => $this->reader->read($pushed->operation)];
            } catch (OperationRejectedException) {
                ++$rejected;
            }
        }

        foreach ($this->inOrder($library, $readable) as ['pushed' => $pushed, 'payload' => $payload]) {
            try {
                $this->accept($library, $pushed, $payload) ? ++$received : ++$known;
            } catch (OperationRejectedException) {
                ++$rejected;
            }
        }

        return new PushResult($received, $known, $rejected);
    }

    /**
     * The link first, so an upload that carries it anywhere in the batch links the library; then by
     * HLC, so a device's registration comes before the operations it signs whatever the batch order.
     *
     * @param list<array{pushed: PushedOperation, payload: OperationPayload}> $readable
     *
     * @return list<array{pushed: PushedOperation, payload: OperationPayload}>
     */
    private function inOrder(CloudLibrary $library, array $readable): array
    {
        usort($readable, fn (array $a, array $b): int => [!$this->isLink($library, $a['payload']), $a['payload']->hlc]
            <=> [!$this->isLink($library, $b['payload']), $b['payload']->hlc]);

        return $readable;
    }

    /**
     * @return bool whether it was stored (false: the log already had it)
     *
     * @throws OperationRejectedException
     */
    private function accept(CloudLibrary $library, PushedOperation $pushed, OperationPayload $payload): bool
    {
        if ($this->operations->has($library, $payload->id)) {
            return false;
        }

        if (!$library->isLinked()) {
            $this->link($library, $pushed, $payload);
        } else {
            if (strtolower($payload->library) !== $library->getLocalLibraryId()) {
                throw new OperationRejectedException(OperationRejection::OtherLibrary);
            }
            $this->verifyAuthor($library, $pushed->operation, $payload);
        }

        return $this->operations->insertIfNew(
            $library,
            $payload->id,
            $this->author($payload),
            $payload->hlc,
            $pushed->schemaVersion,
            json_encode($pushed->operation, \JSON_THROW_ON_ERROR),
        );
    }

    /**
     * @throws OperationRejectedException unless this is the library's own link, signed by a device of its owner
     */
    private function link(CloudLibrary $library, PushedOperation $pushed, OperationPayload $payload): void
    {
        if (!$this->isLink($library, $payload)) {
            throw new OperationRejectedException(OperationRejection::NotLinked);
        }

        // The owner's own account devices, found by the account and the id from the HLC, never by the id alone.
        $device = $this->devices->findOneOfUser($library->getOwner(), $this->author($payload));
        if (!$device instanceof Device || $device->isRemoved()) {
            throw new OperationRejectedException(OperationRejection::UnknownKey);
        }
        $this->verify($pushed->operation, $device->getPublicKey());

        $library->link($payload->library);
        $this->entityManager->flush();
    }

    /**
     * The key is the one registered in this library's log for the author; if
     * there is none, the one the operation registers for its own author, which
     * then stays. A different key for a registered device is refused.
     *
     * @throws OperationRejectedException
     */
    private function verifyAuthor(CloudLibrary $library, \stdClass $operation, OperationPayload $payload): void
    {
        $author = $this->author($payload);
        $registered = $this->deviceKeys->findOneInLibrary($library, $author)?->getPublicKey();
        $declared = $this->declaredKey($payload, $author);

        if (null !== $registered) {
            if (null !== $declared && !hash_equals($registered, $declared)) {
                throw new OperationRejectedException(OperationRejection::KeyChanged);
            }
            $this->verify($operation, $registered);

            return;
        }
        if (null === $declared) {
            throw new OperationRejectedException(OperationRejection::UnknownKey);
        }

        $this->verify($operation, $declared);
        $this->entityManager->persist(new CloudDeviceKey($library, $author, $declared));
        $this->entityManager->flush();
    }

    /**
     * @throws OperationRejectedException
     */
    private function verify(\stdClass $operation, string $publicKey): void
    {
        try {
            $this->signature->verify($operation, $publicKey);
        } catch (InvalidSignatureException) {
            throw new OperationRejectedException(OperationRejection::BadSignature);
        }
    }

    /** The public key a `device` create change in the operation gives its own author, if any. */
    private function declaredKey(OperationPayload $payload, string $author): ?string
    {
        foreach ($payload->changes as $change) {
            $key = $change->fields['public_key'] ?? null;
            if (CatalogEntity::Device === $change->entity && ChangeAction::Create === $change->action
                && $change->id === $author && \is_string($key)) {
                return $key;
            }
        }

        return null;
    }

    private function isLink(CloudLibrary $library, OperationPayload $payload): bool
    {
        foreach ($payload->changes as $change) {
            $cloudId = $change->fields['cloud_library_id'] ?? null;
            if (CatalogEntity::Library === $change->entity && \is_string($cloudId)
                && strtolower($cloudId) === $library->getId()->toRfc4122()) {
                return true;
            }
        }

        return false;
    }

    /** The device in the HLC: what follows the time and the counter. */
    private function author(OperationPayload $payload): string
    {
        return explode('-', $payload->hlc, 3)[2];
    }
}
