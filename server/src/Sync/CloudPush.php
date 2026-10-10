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
use App\Enum\OperationStatus;
use App\Repository\CloudDeviceKeyRepository;
use App\Repository\CloudOperationRepository;
use App\Security\AccountException;
use App\Sync\Permission\CatalogJudge;
use Doctrine\DBAL\LockMode;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;
use Symfony\Component\DependencyInjection\Attribute\Autowire;

/**
 * The rules for what a cloud library accepts (D84; docs/architecture/cloud.md).
 * It judges each operation only by what it says (structure, library, signature),
 * never by when it arrives (seam 7), and it neither reads ids from the client to
 * decide access. What it stores is handed to the CatalogJudge, which applies it or leaves it as a request
 * (milestone 7.4.6); the roles are the judge's business, not this class's. The caller has already checked
 * access (App\Sync\CloudLibraryAccess).
 *
 * Until the library is linked, only the link operation may enter (seam 3):
 * a change of the `library` row naming this library's cloud id, signed by a
 * device of the account that owns it.
 */
final readonly class CloudPush
{
    /** How far ahead of the server's clock an operation may be dated: the app's `MAX_CLOCK_DRIFT_MILLIS` (crates/domain/src/hlc.rs). */
    public const int MAX_CLOCK_DRIFT_MILLIS = 3_600_000;

    public function __construct(
        private EntityManagerInterface $entityManager,
        private SchemaVersion $schemaVersion,
        private OperationReader $reader,
        private OperationSignature $signature,
        private CloudOperationRepository $operations,
        private CloudDeviceKeyRepository $deviceKeys,
        private CloudLink $link,
        private SyncNotifier $notifier,
        private CatalogJudge $judge,
        #[Autowire(lazy: true)]
        private CloudAuthor $author,
        #[Autowire(lazy: true)]
        private AccessCutter $cutter,
        private ClockInterface $clock,
    ) {
    }

    /**
     * @param Device $pushedBy the device the request was authenticated with: who uploaded, never what the request says
     *
     * @throws AccountException when the client's database is on another schema version
     */
    public function push(CloudLibrary $library, PushRequest $request, Device $pushedBy): PushResult
    {
        // A client on another version does not sync at all (D31). What each operation was recorded at is judged
        // one by one in apply(): history from before an update is still good.
        $this->schemaVersion->require($request->schemaVersion);

        $cut = [];
        $result = $this->entityManager->wrapInTransaction(function () use ($library, $request, $pushedBy, &$cut): PushResult {
            $result = $this->applyLocked($library, $request->operations, $pushedBy);
            // The library has just been linked and names its cloud device: the device registers itself, last (D95).
            // Not counted in the answer: the counts are the client's operations.
            $this->author->registerIfNamed($library);
            // A withdrawal that has just been judged cuts the account of that person, now (D98), after `settle`.
            if ($result->received > 0) {
                $cut = $this->cutter->cut($library);
            }

            return $result;
        });

        // After the commit, and only when something new was stored (D75).
        if ($result->received > 0) {
            $this->notifier->libraryChanged($library);
        }
        // The accounts that have just lost the library hear it on their own topic, to sync and be told.
        foreach ($cut as $user) {
            $this->notifier->librariesOfAccountChanged($user);
        }

        return $result;
    }

    /**
     * Stores and judges the operations, inside the transaction the caller has opened. The same path for what a
     * device uploads and for what the cloud signs itself (CloudAuthor): no operation enters by another one.
     *
     * @param list<PushedOperation> $operations
     * @param ?Device               $pushedBy   the device the upload was authenticated with; null for what the server itself signs
     */
    public function applyLocked(CloudLibrary $library, array $operations, ?Device $pushedBy): PushResult
    {
        // One push at a time per library: linking and registering a key check first and write after.
        // The refresh also brings the link state as it is now, not as it was when the library was loaded.
        $this->entityManager->refresh($library, LockMode::PESSIMISTIC_WRITE);

        $entries = [];
        foreach ($operations as $pushed) {
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
                if ($this->accept($library, $pushed, $payload, $verifiedLink, $pushedBy)) {
                    ++$received;
                    // In HLC order, as `take` needs; a stored operation is judged even when it ends as a request.
                    $this->judge->take($library, $payload);
                } else {
                    ++$known;
                }
            } catch (OperationRejectedException) {
                ++$rejected;
                if (null !== $author) {
                    $heldBack[$author] = true;
                }
            }
        }

        $this->judge->settle($library);
        $this->requireLinkAllowed($library);

        return new PushResult($received, $known, $rejected);
    }

    /**
     * The library's own log must not contradict its link (D93): if the operation that links it ended as a
     * request, its author could not link it, and this push is undone whole. Checked in every push, because
     * one that brings the roles can turn a link applied before into a request.
     *
     * @throws AccountException `link_not_allowed`, which undoes the push's transaction
     */
    private function requireLinkAllowed(CloudLibrary $library): void
    {
        $link = $library->getLinkOperationId()?->toRfc4122();
        if (null !== $link && OperationStatus::Request === ($this->operations->statusesOf($library, [$link])[$link] ?? null)) {
            throw AccountException::linkNotAllowed();
        }
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
     * @return ?PushedOperation that operation, already verified; null when none links the library
     */
    private function linkWith(CloudLibrary $library, array $entries): ?PushedOperation
    {
        foreach ($entries as ['pushed' => $pushed, 'payload' => $payload]) {
            if (!$payload instanceof OperationPayload || !$this->link->isLink($library, $payload)) {
                continue;
            }
            try {
                $this->link->link($library, $pushed, $payload);

                return $pushed;
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
    private function accept(CloudLibrary $library, PushedOperation $pushed, OperationPayload $payload, ?PushedOperation $verifiedLink, ?Device $pushedBy): bool
    {
        if ($this->operations->has($library, $payload->id)) {
            return false;
        }

        // Nothing dated further ahead than a device's clock may drift (`MAX_CLOCK_DRIFT_MILLIS` in the app, one hour):
        // such an operation lifts the floor of every HLC the library writes afterwards, the cloud's own included (SEG-028).
        if ((int) substr($payload->hlc, 0, 16) > (int) $this->clock->now()->format('Uv') + self::MAX_CLOCK_DRIFT_MILLIS) {
            throw new OperationRejectedException(OperationRejection::TooFarAhead);
        }
        if (!$library->isLinked()) {
            // Nothing enters before the link.
            throw new OperationRejectedException(OperationRejection::NotLinked);
        }
        if (strtolower($payload->library) !== $library->getLocalLibraryId()) {
            throw new OperationRejectedException(OperationRejection::OtherLibrary);
        }
        // The entry that just linked the library was verified with its account's key, and only that entry: an id
        // says nothing about who wrote an operation (SEG-022).
        $isTheLink = $pushed === $verifiedLink;
        $remembered = $library->getLinkOperationId()?->toRfc4122();
        if (!$isTheLink && strtolower($payload->id) === $remembered && !$this->link->holds($library, $pushed, $payload)) {
            // The id remembered as the link's is the link's alone, or D93 would look at another operation.
            throw new OperationRejectedException(OperationRejection::BadSignature);
        }
        // A device's key never changes, whoever signs the change (as in the app's signature_holds).
        $this->link->refuseKeyChanges($library, $payload);
        if (!$isTheLink) {
            $this->verifyAuthor($library, $pushed->operation, $payload);
        }
        $this->requirePrevious($library, $payload);
        if ($this->operations->hasHlc($library, $payload->hlc)) {
            throw new OperationRejectedException(OperationRejection::HlcTaken);
        }

        $stored = $this->operations->insertIfNew(
            $library,
            $payload->id,
            $this->author($payload),
            $payload->hlc,
            $pushed->schemaVersion,
            json_encode($pushed->operation, \JSON_THROW_ON_ERROR),
            $payload->previous,
            $pushedBy?->getUser()->getId()->toRfc4122(),
            $pushedBy?->getDeviceId(),
        );
        // A library linked before the link was remembered (7.4.6) learns it when its link arrives (SEG-024).
        if ($stored && null === $remembered && $this->link->holds($library, $pushed, $payload)) {
            $this->link->note($library, $payload);
        }

        return $stored;
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

    /** The device in the HLC: what follows the time and the counter. */
    private function author(OperationPayload $payload): string
    {
        return explode('-', $payload->hlc, 3)[2];
    }
}
