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
        private SyncNotifier $notifier,
    ) {
    }

    /**
     * @throws AccountException when the client's database is on another schema version
     */
    public function push(CloudLibrary $library, PushRequest $request): PushResult
    {
        // A client on another version does not sync at all (D31). What each operation was recorded at is judged
        // one by one in apply(): history from before an update is still good.
        $this->schemaVersion->require($request->schemaVersion);

        $result = $this->entityManager->wrapInTransaction(fn (): PushResult => $this->apply($library, $request));

        // After the commit, and only when something new was stored (D75).
        if ($result->received > 0) {
            $this->notifier->libraryChanged($library);
        }

        return $result;
    }

    private function apply(CloudLibrary $library, PushRequest $request): PushResult
    {
        // One push at a time per library: linking and registering a key check first and write after.
        // The refresh also brings the link state as it is now, not as it was when the library was loaded.
        $this->entityManager->refresh($library, LockMode::PESSIMISTIC_WRITE);

        $entries = [];
        foreach ($request->operations as $pushed) {
            $entries[] = $this->read($pushed);
        }

        // An unlinked library takes the link first, so an upload that carries it anywhere in the batch links the
        // library. The link itself is stored in its place by HLC, after the operations it chains to (D87).
        $verifiedLink = $library->isLinked() ? null : $this->linkWith($library, $entries);

        // As in the app's merge_in: once an author's operation is rejected, that author's later ones in this
        // push are too, so the library's vector never jumps over a gap that nobody would send again (DAT-001).
        $received = $known = $rejected = 0;
        $heldBack = [];
        foreach ($this->inHlcOrder($entries) as ['pushed' => $pushed, 'payload' => $payload, 'author' => $author]) {
            if (null !== $author && isset($heldBack[$author])) {
                ++$rejected;
                continue;
            }
            try {
                if (!$payload instanceof OperationPayload) {
                    throw new OperationRejectedException(OperationRejection::Malformed);
                }
                $this->accept($library, $pushed, $payload, $verifiedLink) ? ++$received : ++$known;
            } catch (OperationRejectedException) {
                ++$rejected;
                if (null !== $author) {
                    $heldBack[$author] = true;
                }
            }
        }

        return new PushResult($received, $known, $rejected);
    }

    /**
     * What the push says about one operation. An unreadable one keeps the HLC and author it claims, if it
     * claims any, only to be sorted and to hold back its author's later operations.
     *
     * @return array{pushed: PushedOperation, payload: ?OperationPayload, hlc: string, author: ?string}
     */
    private function read(PushedOperation $pushed): array
    {
        $claimed = $pushed->operation->hlc ?? null;
        $hlc = \is_string($claimed) ? $claimed : '';
        $parts = explode('-', $hlc, 3);
        $author = 3 === \count($parts) && '' !== $parts[2] ? $parts[2] : null;

        try {
            // Older history is fine; a version this server does not know cannot be judged.
            if ($pushed->schemaVersion > $this->schemaVersion->current()) {
                throw new OperationRejectedException(OperationRejection::NewerVersion);
            }
            $payload = $this->reader->read($pushed->operation);

            return ['pushed' => $pushed, 'payload' => $payload, 'hlc' => $payload->hlc, 'author' => $this->author($payload)];
        } catch (OperationRejectedException) {
            return ['pushed' => $pushed, 'payload' => null, 'hlc' => $hlc, 'author' => $author];
        }
    }

    /**
     * By HLC, so a device's registration comes before the operations it signs, and each operation after the
     * previous one of its author, whatever the batch order.
     *
     * @param list<array{pushed: PushedOperation, payload: ?OperationPayload, hlc: string, author: ?string}> $entries
     *
     * @return list<array{pushed: PushedOperation, payload: ?OperationPayload, hlc: string, author: ?string}>
     */
    private function inHlcOrder(array $entries): array
    {
        usort($entries, static fn (array $a, array $b): int => $a['hlc'] <=> $b['hlc']);

        return $entries;
    }

    /**
     * Links the library with the first operation of the batch that is a valid link, without storing it.
     *
     * @param list<array{pushed: PushedOperation, payload: ?OperationPayload, hlc: string, author: ?string}> $entries
     *
     * @return ?string the id of that operation, already verified; null when none links the library
     */
    private function linkWith(CloudLibrary $library, array $entries): ?string
    {
        foreach ($entries as ['pushed' => $pushed, 'payload' => $payload]) {
            if (!$payload instanceof OperationPayload || !$this->isLink($library, $payload)) {
                continue;
            }
            try {
                $this->link($library, $pushed, $payload);

                return $payload->id;
            } catch (OperationRejectedException) {
                // Judged again, and counted, in its place.
            }
        }

        return null;
    }

    /**
     * @return bool whether it was stored (false: the log already had it)
     *
     * @throws OperationRejectedException
     */
    private function accept(CloudLibrary $library, PushedOperation $pushed, OperationPayload $payload, ?string $verifiedLink): bool
    {
        if ($this->operations->has($library, $payload->id)) {
            return false;
        }

        if (!$library->isLinked()) {
            // Nothing enters before the link.
            throw new OperationRejectedException(OperationRejection::NotLinked);
        }
        if (strtolower($payload->library) !== $library->getLocalLibraryId()) {
            throw new OperationRejectedException(OperationRejection::OtherLibrary);
        }
        if ($payload->id !== $verifiedLink) {
            $this->verifyAuthor($library, $pushed->operation, $payload);
        }
        $this->requirePrevious($library, $payload);
        if ($this->operations->hasHlc($library, $payload->hlc)) {
            throw new OperationRejectedException(OperationRejection::HlcTaken);
        }

        return $this->operations->insertIfNew(
            $library,
            $payload->id,
            $this->author($payload),
            $payload->hlc,
            $pushed->schemaVersion,
            json_encode($pushed->operation, \JSON_THROW_ON_ERROR),
            $payload->previous,
        );
    }

    /**
     * An operation follows an operation of its author that this library already stores (D87): one whose previous
     * is missing waits for it, with the rest of its author's (the caller holds them back); one that chains to
     * another author's operation or to a later one is malformed. A null previous is taken as it comes.
     *
     * @throws OperationRejectedException
     */
    private function requirePrevious(CloudLibrary $library, OperationPayload $payload): void
    {
        if (null === $payload->previous) {
            return;
        }
        $previous = $this->operations->findAuthorAndHlc($library, $payload->previous);
        if (null === $previous || $previous['author'] !== $this->author($payload) || $previous['hlc'] >= $payload->hlc) {
            throw new OperationRejectedException(OperationRejection::BrokenChain);
        }
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

        // A device's key never changes, whoever signs the change (as in the app's signature_holds).
        $this->refuseKeyChanges($library, $payload);

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

    /**
     * @throws OperationRejectedException when a `device` change offers another key than the registered one
     */
    private function refuseKeyChanges(CloudLibrary $library, OperationPayload $payload): void
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
            if (CatalogEntity::Library === $change->entity && ChangeAction::Update === $change->action
                && strtolower($change->id) === strtolower($payload->library)
                && \is_string($cloudId) && strtolower($cloudId) === $library->getId()->toRfc4122()) {
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
