<?php

declare(strict_types=1);

namespace App\Security;

use App\Dto\OperationPayload;
use App\Entity\ActivityEntry;
use App\Entity\Catalog\Library;
use App\Entity\CloudLibrary;
use App\Entity\Device;
use App\Entity\User;
use App\Enum\ActivityKind;
use App\Repository\MembershipRepository;
use App\Sync\Invitation\EmailFingerprint;
use App\Sync\Permission\CatalogLibraryView;
use Doctrine\ORM\EntityManagerInterface;
use Psr\Log\LoggerInterface;
use Symfony\Component\Clock\ClockInterface;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\RequestStack;
use Symfony\Component\Uid\Uuid;

/**
 * The only piece that writes the activity log (D66, 7.7), called from where each thing happens: nothing listens to
 * every request. What a line says about **who**, **when** and **where** comes from the server and never from what a
 * client sent (D84): the account and device are those of the session, the time is the server's clock, the address is
 * the one Symfony trusts according to `TRUSTED_PROXIES`, the library is the one the table of access gave and its
 * name is the one the server holds. What a client's body decides is only the kind of an operation's event, and only
 * once the operation has been applied.
 *
 * **Inside the caller's work.** `record` only persists: the line is written by the flush that writes the thing it
 * records, in the same transaction wherever there is one (an operation, an invitation, a password change), so that
 * neither exists without the other. Where nothing else is written (a failed login) `recordNow` flushes, and if that
 * fails it says so in the log of the application and lets the answer go on: a failed login gives access to nobody,
 * and the log failing must not turn it into an error that tells an attacker which account exists.
 */
final readonly class ActivityLog
{
    private const int USER_AGENT_LENGTH = 255;
    private const int NAME_LENGTH = 200;

    public function __construct(
        private EntityManagerInterface $entityManager,
        private ClockInterface $clock,
        private RequestStack $requestStack,
        private EmailFingerprint $fingerprints,
        private LoggerInterface $logger,
        private MembershipRepository $memberships,
    ) {
    }

    /**
     * @param ?string $libraryName the name the server had, if the caller already has it; otherwise it is read from the catalog
     * @param ?string $detail      a short closed value: a role, `true`, the kind of invitation or the id of a person; never free text
     */
    public function record(
        ActivityKind $kind,
        ?User $account = null,
        ?string $deviceId = null,
        ?CloudLibrary $library = null,
        ?string $subject = null,
        ?string $operation = null,
        ?string $detail = null,
        ?string $libraryName = null,
    ): void {
        $this->persist($kind, $account, $deviceId, $library, $library instanceof CloudLibrary ? $libraryName ?? $this->nameOf($library) : (null), $subject, $operation, $detail);
    }

    /** For what has no other write to ride on: flushes, and never lets a failure of the log change the answer. */
    public function recordNow(ActivityKind $kind, ?User $account = null): void
    {
        try {
            $this->record($kind, $account);
            $this->entityManager->flush();
        } catch (\Throwable $exception) {
            $this->logger->error('The activity log could not be written.', ['kind' => $kind->value, 'exception' => $exception]);
        }
    }

    /**
     * What applied operations say, written in the transaction of the push that applied them.
     *
     * @param list<array{operation: OperationPayload, events: list<array{kind: ActivityKind, subject: ?string, detail: ?string}>}> $applied
     * @param ?Device                                                                                                              $by      the device the upload was authenticated with; null for what the server itself signs
     */
    public function recordOperations(CloudLibrary $library, array $applied, ?Device $by): void
    {
        if ([] === $applied) {
            return;
        }
        $name = $this->nameOf($library);
        $view = new CatalogLibraryView($this->entityManager, $library);
        foreach ($applied as ['operation' => $operation, 'events' => $events]) {
            // The author is the device whose signature the server verified; the uploader is only whose session carried it (SEG-041).
            $author = $operation->author();
            foreach ($events as $event) {
                $account = $by instanceof Device ? $this->accountOfDevice($library, $view, $author) : $this->accountOfCloudEvent($library, $event);
                $this->persist($event['kind'], $account, $author, $library, $name, $event['subject'], $operation->id, $event['detail'], $by?->getUser(), $by?->getDeviceId());
            }
        }
    }

    /** The account of the person this device belongs to in the library, if the person has one (the server's own tables, D96). */
    private function accountOfDevice(CloudLibrary $library, CatalogLibraryView $view, string $device): ?User
    {
        $member = $view->memberOfDevice($device);

        return null === $member || !Uuid::isValid($member) ? null : $this->memberships->findOfMember($library, Uuid::fromString($member))?->getUser();
    }

    /**
     * What the cloud does by itself is done for an account: the one that is that person (read from the server's own table).
     *
     * @param array{kind: ActivityKind, subject: ?string, detail: ?string} $event
     */
    private function accountOfCloudEvent(CloudLibrary $library, array $event): ?User
    {
        $member = match ($event['kind']) {
            ActivityKind::AccountLinked => $event['subject'],
            ActivityKind::DeviceLinked => $event['detail'],
            default => null,
        };

        return null === $member || !Uuid::isValid($member) ? null : $this->memberships->findOfMember($library, Uuid::fromString($member))?->getUser();
    }

    private function persist(ActivityKind $kind, ?User $account, ?string $deviceId, ?CloudLibrary $library, ?string $libraryName, ?string $subject, ?string $operation, ?string $detail, ?User $uploader = null, ?string $uploaderDevice = null): void
    {
        $request = $this->requestStack->getCurrentRequest();
        $this->entityManager->persist(new ActivityEntry(
            $kind,
            $this->clock->now()->setTimezone(new \DateTimeZone('UTC')),
            $request instanceof Request ? 'http' : 'console',
            $account?->getId(),
            $account instanceof User ? $this->fingerprints->of($account->getEmail()) : null,
            $deviceId,
            $library?->getId(),
            $this->clean($libraryName, self::NAME_LENGTH),
            $subject,
            null === $operation || !Uuid::isValid($operation) ? null : Uuid::fromString($operation),
            $detail,
            $this->addressOf($request?->getClientIp()),
            $this->clean($request?->headers->get('User-Agent'), self::USER_AGENT_LENGTH),
            $uploader?->getId(),
            $uploaderDevice,
        ));
    }

    /** The name of the library in the server's own catalog now, or null if it has none (yet, or any more). */
    private function nameOf(CloudLibrary $library): ?string
    {
        $row = $this->entityManager->createQuery(\sprintf('SELECT l.name AS name FROM %s l WHERE l.cloudLibrary = :library', Library::class))
            ->setParameter('library', $library->getId(), 'uuid')->setMaxResults(1)->getOneOrNullResult();

        return \is_array($row) && \is_string($row['name']) ? $row['name'] : null;
    }

    /** An address as `getClientIp` gives it (it already honours only the proxies that are trusted), and only if it is one. */
    private function addressOf(?string $address): ?string
    {
        return null !== $address && false !== filter_var($address, \FILTER_VALIDATE_IP) ? $address : null;
    }

    /**
     * Text a client chose, made safe to keep and to print: valid UTF-8, no control characters (a line break or an escape
     * sequence would write over a console or forge a line of a log), cut to a fixed length. The cut comes first, so a
     * header of megabytes costs no more than one of a few hundred characters.
     */
    private function clean(?string $text, int $length): ?string
    {
        if (null === $text) {
            return null;
        }
        $text = mb_substr(mb_scrub($text), 0, $length * 2);
        $text = trim((string) preg_replace('/[\x00-\x1F\x7F\x{80}-\x{9F}\x{2028}\x{2029}]+/u', ' ', $text));
        $text = mb_substr($text, 0, $length);

        return '' === $text ? null : $text;
    }
}
