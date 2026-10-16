<?php

declare(strict_types=1);

namespace App\Sync;

use App\Dto\PushedOperation;
use App\Entity\CloudLibrary;
use App\Repository\CloudOperationRepository;
use App\Security\CloudKeyVault;
use App\Security\CloudKeyVaultException;
use App\Sync\Permission\CatalogLibraryView;
use Doctrine\DBAL\LockMode;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;
use Symfony\Component\DependencyInjection\Attribute\Autowire;
use Symfony\Component\Uid\Uuid;

/**
 * The operations the cloud writes itself, signed with the key of its device in the library (D76, D95).
 *
 * It builds the operation as the app does, signs it with the cloud's private key (opened with the vault only
 * for that), and stores and judges it through `CloudPush::applyLocked`, the same path as an upload: it is read
 * with the OperationReader, its signature is checked against the key registered for the cloud's id, it must
 * chain to the cloud's previous operation in that library, its HLC must be unique, and the judge takes it.
 * Nothing here decides whether the cloud may do it: the permission rules do, and without the power (the
 * library has not named it, or has withdrawn it) the operation ends as a request.
 *
 * **Nobody calls it yet.** 7.5.2b will use it to write `member.account_id` and to link devices of an account to
 * their person, and 7.5.3 for the new phone of an account. The caller opens a transaction (this class takes the
 * library lock inside it, as a push does) and, after the commit, tells the clients (`SyncNotifier::libraryChanged`).
 */
final readonly class CloudAuthor
{
    public function __construct(
        private EntityManagerInterface $entityManager,
        private CloudKeyVault $vault,
        private CloudPush $push,
        private CloudOperationRepository $operations,
        private OperationSigner $signer,
        private SchemaVersion $schemaVersion,
        private ClockInterface $clock,
        #[Autowire(env: 'DEFAULT_URI')]
        private string $serverUri,
    ) {
    }

    /**
     * @param list<array<string, mixed>> $changes the changes as an operation carries them (`entity`, `id`, `action`, `fields`)
     *
     * @return string the id of the operation, stored and judged
     *
     * @throws CloudAuthorException without a transaction, without identity, with the library not linked, or when the log refuses it
     */
    public function author(CloudLibrary $library, string $intent, array $changes): string
    {
        if (!$this->entityManager->getConnection()->isTransactionActive()) {
            throw CloudAuthorException::noTransaction();
        }
        // The lock of every push; the refresh also brings the link and the identity as they are now.
        $this->entityManager->refresh($library, LockMode::PESSIMISTIC_WRITE);
        $device = $library->getCloudDeviceId();
        $sealed = $library->getCloudSecretKey();
        $local = $library->getLocalLibraryId();
        if (!$device instanceof Uuid || null === $sealed || !$this->vault->isEnabled()) {
            throw CloudAuthorException::noIdentity();
        }
        if (null === $local) {
            throw CloudAuthorException::notLinked();
        }

        $previous = $this->operations->latestOf($library, $device->toRfc4122());
        $newest = max([...array_values($this->operations->vectorOf($library)), $previous['hlc'] ?? '']);
        $operation = [
            'id' => Uuid::v7()->toRfc4122(),
            'library' => $local,
            'hlc' => $this->hlc($device->toRfc4122(), '' === $newest ? null : $newest),
            'intent' => $intent,
            'changes' => $changes,
            'previous' => $previous['id'] ?? null,
        ];
        try {
            $secretKey = $this->vault->open($library->getId()->toRfc4122(), $sealed);
        } catch (CloudKeyVaultException) {
            // Another secret than the one the key was sealed with: the cloud cannot sign here, and says so as such.
            throw CloudAuthorException::noIdentity();
        }
        try {
            $operation['signature'] = $this->signer->sign($operation, $secretKey);
        } finally {
            sodium_memzero($secretKey);
        }

        $body = json_decode(json_encode($operation, \JSON_THROW_ON_ERROR), false, flags: \JSON_THROW_ON_ERROR);
        \assert($body instanceof \stdClass);
        if (1 !== $this->push->applyLocked($library, [new PushedOperation($this->schemaVersion->current(), $body)], null)->received) {
            throw CloudAuthorException::refused();
        }

        return $operation['id'];
    }

    /**
     * The cloud's device id if it has the power in this library now (D76, D95): the device the catalog names, registered
     * with the key its id derives from. Without it what the cloud signs would only be a request.
     */
    public function power(CloudLibrary $library): ?Uuid
    {
        $cloud = $library->getCloudDeviceId();
        $named = new CatalogLibraryView($this->entityManager, $library)->cloudDevice();

        return $cloud instanceof Uuid && null !== $named && strtolower($named) === $cloud->toRfc4122() ? $cloud : null;
    }

    /**
     * Registers the cloud device in the library's log (D95): a `device.register` signed with its own key, with its
     * own id in the HLC, as any device registers (spec/README.md allows no other way to create a `device` row). It
     * is written once, when the library's catalog names this device in `cloud_device_id` and the device has no
     * operation yet: at the end of the push that brings the naming, so it is the newest in the log. If the
     * administrator withdraws the power and names it again, nothing more is written.
     *
     * Inside the push's transaction and lock, after `settle`. Does nothing without identity or link.
     *
     * @return ?string the id of the operation, or null when nothing was written
     */
    public function registerIfNamed(CloudLibrary $library): ?string
    {
        $device = $library->getCloudDeviceId();
        $publicKey = $library->getCloudPublicKey();
        if (!$device instanceof Uuid || null === $publicKey || !$library->isLinked() || !$this->vault->isEnabled()) {
            return null;
        }
        $named = new CatalogLibraryView($this->entityManager, $library)->namedCloudDevice();
        if (null === $named || strtolower($named) !== $device->toRfc4122() || null !== $this->operations->latestOf($library, $device->toRfc4122())) {
            return null;
        }

        return $this->author($library, 'device.register', [[
            'entity' => 'device',
            'id' => $device->toRfc4122(),
            'action' => 'create',
            'fields' => ['public_key' => $publicKey, 'name' => (string) (parse_url($this->serverUri, \PHP_URL_HOST) ?: 'cloud')],
        ]]);
    }

    /**
     * The server's time in milliseconds, never below the newest operation of the library (its own previous one
     * included), so what it writes is not «late» for the judge: when the clock has not moved past it (or went
     * back), the time stays and the counter goes up. The format of the app's `Hlc`.
     */
    private function hlc(string $device, ?string $previous): string
    {
        $millis = (int) $this->clock->now()->format('Uv');
        $counter = 0;
        if (null !== $previous) {
            [$time, $count] = explode('-', $previous, 3);
            if ((int) $time >= $millis) {
                $millis = (int) $time;
                $counter = (int) hexdec($count) + 1;
            }
        }
        // A full counter moves on to the next millisecond (SEG-028): eight hexadecimal digits is all the format has.
        if ($counter > 0xFFFFFFFF) {
            ++$millis;
            $counter = 0;
        }
        if ($millis > 9_999_999_999_999_999) {
            throw CloudAuthorException::refused();
        }

        return \sprintf('%016d-%08x-%s', $millis, $counter, $device);
    }
}
