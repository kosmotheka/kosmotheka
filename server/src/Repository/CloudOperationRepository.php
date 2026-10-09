<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\Catalog\FenceChain;
use App\Entity\CloudLibrary;
use App\Entity\CloudOperation;
use App\Enum\OperationStatus;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\DBAL\ArrayParameterType;
use Doctrine\Persistence\ManagerRegistry;
use Symfony\Component\Uid\NilUuid;
use Symfony\Component\Uid\Uuid;

/**
 * The operation log of the cloud libraries. SQL rather than the ORM, for the
 * reasons in App\Entity\CloudOperation.
 *
 * @extends ServiceEntityRepository<CloudOperation>
 */
final class CloudOperationRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, CloudOperation::class);
    }

    /** Whether this library's log has the operation. The id is only ever looked up inside one library (D84). */
    public function has(CloudLibrary $library, string $operationId): bool
    {
        return false !== $this->getEntityManager()->getConnection()->fetchOne(
            'SELECT 1 FROM cloud_operation WHERE cloud_library_id = :library AND operation_id = :operation',
            ['library' => $library->getId()->toRfc4122(), 'operation' => $operationId],
        );
    }

    /**
     * The author and HLC of an operation of this library's log, or null if it has none with that id.
     *
     * @return array{author: string, hlc: string}|null
     */
    public function findAuthorAndHlc(CloudLibrary $library, string $operationId): ?array
    {
        $row = $this->getEntityManager()->getConnection()->fetchAssociative(
            'SELECT author_device_id, hlc FROM cloud_operation WHERE cloud_library_id = :library AND operation_id = :operation',
            ['library' => $library->getId()->toRfc4122(), 'operation' => $operationId],
        );

        if (false === $row) {
            return null;
        }
        \assert(\is_string($row['author_device_id']) && \is_string($row['hlc']));

        return ['author' => $row['author_device_id'], 'hlc' => $row['hlc']];
    }

    /** Whether another operation of this library holds the HLC (unique per library: it names its device). */
    public function hasHlc(CloudLibrary $library, string $hlc): bool
    {
        return false !== $this->getEntityManager()->getConnection()->fetchOne(
            'SELECT 1 FROM cloud_operation WHERE cloud_library_id = :library AND hlc = :hlc',
            ['library' => $library->getId()->toRfc4122(), 'hlc' => $hlc],
        );
    }

    /**
     * Stores the operation as received, unless the library's log already has it (then nothing changes, who uploaded it included).
     *
     * @return bool whether it was stored (false: it was already known)
     */
    public function insertIfNew(CloudLibrary $library, string $operationId, string $authorDeviceId, string $hlc, int $schemaVersion, string $bodyJson, ?string $previous = null, ?string $pushedByUserId = null, ?string $pushedByDeviceId = null): bool
    {
        $inserted = $this->getEntityManager()->getConnection()->executeStatement(
            'INSERT INTO cloud_operation (cloud_library_id, operation_id, author_device_id, hlc, schema_version, body, received_at, previous, pushed_by_user_id, pushed_by_device_id)
             VALUES (:library, :operation, :author, :hlc, :schema, CAST(:body AS JSONB), :received, :previous, :pushedByUser, :pushedByDevice)
             ON CONFLICT (cloud_library_id, operation_id) DO NOTHING',
            [
                'library' => $library->getId()->toRfc4122(),
                'operation' => $operationId,
                'author' => $authorDeviceId,
                'hlc' => $hlc,
                'schema' => $schemaVersion,
                'body' => $bodyJson,
                'received' => new \DateTimeImmutable('now', new \DateTimeZone('UTC'))->format('Y-m-d H:i:s'),
                'previous' => $previous,
                'pushedByUser' => $pushedByUserId,
                'pushedByDevice' => $pushedByDeviceId,
            ],
        );

        return 1 === $inserted;
    }

    /**
     * The operations whose HLC is greater than the vector's entry for their
     * author (an author the vector does not name comes in full), by HLC.
     *
     * @param array<string, string> $vector the last HLC the client has per author
     *
     * @return list<array{schemaVersion: int, operation: \stdClass}>
     */
    public function findSince(CloudLibrary $library, array $vector): array
    {
        $connection = $this->getEntityManager()->getConnection();
        $query = $connection->createQueryBuilder()
            ->select('schema_version', 'CAST(body AS TEXT) AS body')
            ->from('cloud_operation')
            ->where('cloud_library_id = :library')
            ->setParameter('library', $library->getId()->toRfc4122())
            ->orderBy('hlc')
            ->addOrderBy('operation_id');

        if ([] !== $vector) {
            $newer = [$query->expr()->notIn('author_device_id', ':known')];
            $query->setParameter('known', array_map(strval(...), array_keys($vector)), ArrayParameterType::STRING);
            $index = 0;
            foreach ($vector as $author => $hlc) {
                $newer[] = $query->expr()->and("author_device_id = :author_$index", "hlc > :hlc_$index");
                $query->setParameter("author_$index", (string) $author)->setParameter("hlc_$index", $hlc);
                ++$index;
            }
            $query->andWhere($query->expr()->or(...$newer));
        }

        $operations = [];
        foreach ($query->executeQuery()->fetchAllAssociative() as $row) {
            \assert(\is_string($row['body']) && is_numeric($row['schema_version']));
            $operation = json_decode($row['body'], false, flags: \JSON_THROW_ON_ERROR);
            \assert($operation instanceof \stdClass);
            $operations[] = ['schemaVersion' => (int) $row['schema_version'], 'operation' => $operation];
        }

        return $operations;
    }

    /**
     * A stretch of the library's log in HLC order: the operations after `$afterHlc` ('' for the start).
     * Read by stretches so a big library never has to fit in memory (the catalog rebuild).
     *
     * @return list<array{hlc: string, operation: \stdClass}>
     */
    public function findPage(CloudLibrary $library, string $afterHlc, int $limit): array
    {
        $rows = $this->getEntityManager()->getConnection()->fetchAllAssociative(
            'SELECT hlc, CAST(body AS TEXT) AS body FROM cloud_operation
              WHERE cloud_library_id = :library AND hlc > :after ORDER BY hlc LIMIT :limit',
            ['library' => $library->getId()->toRfc4122(), 'after' => $afterHlc, 'limit' => $limit],
        );

        $page = [];
        foreach ($rows as $row) {
            \assert(\is_string($row['hlc']) && \is_string($row['body']));
            $operation = json_decode($row['body'], false, flags: \JSON_THROW_ON_ERROR);
            \assert($operation instanceof \stdClass);
            $page[] = ['hlc' => $row['hlc'], 'operation' => $operation];
        }

        return $page;
    }

    /**
     * The library's own vector: the greatest HLC of each author.
     *
     * @return array<string, string>
     */
    public function vectorOf(CloudLibrary $library): array
    {
        $rows = $this->getEntityManager()->getConnection()->fetchAllKeyValue(
            'SELECT author_device_id, MAX(hlc) FROM cloud_operation WHERE cloud_library_id = :library GROUP BY author_device_id',
            ['library' => $library->getId()->toRfc4122()],
        );

        $vector = [];
        foreach ($rows as $author => $hlc) {
            \assert(\is_string($hlc));
            $vector[(string) $author] = $hlc;
        }

        return $vector;
    }

    /**
     * The greatest HLC among the operations the permission rules already judged (D73): what the library
     * had before the one being judged now. Null when none was judged.
     */
    public function newestJudgedHlc(CloudLibrary $library): ?string
    {
        $newest = $this->getEntityManager()->createQuery(\sprintf('SELECT MAX(o.hlc) FROM %s o WHERE o.cloudLibrary = :library AND o.status IS NOT NULL', CloudOperation::class))
            ->setParameter('library', $library->getId(), 'uuid')->getSingleScalarResult();

        return \is_string($newest) ? $newest : null;
    }

    public function setStatus(CloudLibrary $library, string $operationId, OperationStatus $status): void
    {
        $this->getEntityManager()->createQuery(\sprintf('UPDATE %s o SET o.status = :status WHERE o.cloudLibrary = :library AND o.operationId = :id', CloudOperation::class))
            ->setParameter('status', $status)->setParameter('library', $library->getId(), 'uuid')->setParameter('id', Uuid::fromString($operationId), 'uuid')->execute();
    }

    /**
     * What each of the operations came to, by id; an id without a judged operation is left out.
     *
     * @param list<string> $operationIds
     *
     * @return array<string, OperationStatus>
     */
    public function statusesOf(CloudLibrary $library, array $operationIds): array
    {
        if ([] === $operationIds) {
            return [];
        }
        $rows = $this->getEntityManager()->createQuery(\sprintf('SELECT o.operationId AS id, o.status AS status FROM %s o WHERE o.cloudLibrary = :library AND o.operationId IN (:ids)', CloudOperation::class))
            ->setParameter('library', $library->getId(), 'uuid')->setParameter('ids', $operationIds, ArrayParameterType::STRING)->getScalarResult();

        $statuses = [];
        foreach ($rows as $row) {
            // A scalar query gives the raw column values: text for the uuid and for the status.
            if (\is_array($row) && \is_string($row['id']) && \is_string($row['status']) && null !== ($status = OperationStatus::tryFrom($row['status']))) {
                $statuses[Uuid::fromString($row['id'])->toRfc4122()] = $status;
            }
        }

        return $statuses;
    }

    /**
     * How many operations of the library ended each way.
     *
     * @return array{applied: int, request: int}
     */
    public function countByStatus(CloudLibrary $library): array
    {
        $count = fn (OperationStatus $status): int => (int) $this->getEntityManager()->createQuery(\sprintf('SELECT COUNT(o.operationId) FROM %s o WHERE o.cloudLibrary = :library AND o.status = :status', CloudOperation::class))
            ->setParameter('library', $library->getId(), 'uuid')->setParameter('status', $status)->getSingleScalarResult();

        return ['applied' => $count(OperationStatus::Applied), 'request' => $count(OperationStatus::Request)];
    }

    /**
     * The operations of one author in the library with the one they chain to (D87), by id.
     *
     * @return array<string, ?string> operation id => its previous
     */
    public function previousOf(CloudLibrary $library, string $deviceId): array
    {
        $rows = $this->getEntityManager()->createQuery(\sprintf('SELECT o.operationId AS id, o.previous AS previous FROM %s o WHERE o.cloudLibrary = :library AND o.authorDeviceId = :device', CloudOperation::class))
            ->setParameter('library', $library->getId(), 'uuid')->setParameter('device', $deviceId)->getScalarResult();

        $chain = [];
        foreach ($rows as $row) {
            if (\is_array($row) && \is_string($row['id'])) {
                $chain[Uuid::fromString($row['id'])->toRfc4122()] = \is_string($row['previous']) ? Uuid::fromString($row['previous'])->toRfc4122() : null;
            }
        }

        return $chain;
    }

    /**
     * Whether the library's log holds an operation of the device that a fence puts under suspicion: dated
     * before it, neither `through` nor in the chain noted for the fence. Judged before the fence was known,
     * it has to be judged again.
     */
    public function hasOperationUnderFence(CloudLibrary $library, string $deviceId, string $until, ?Uuid $through, Uuid $fenceOperation): bool
    {
        $found = $this->getEntityManager()->createQuery(\sprintf(
            'SELECT 1 FROM %s o WHERE o.cloudLibrary = :library AND o.authorDeviceId = :device AND o.hlc < :until AND o.operationId <> :through
               AND NOT EXISTS (SELECT 1 FROM %s c WHERE c.cloudLibrary = :library AND c.fenceOperationId = :fence AND c.deviceId = :device AND c.operationId = o.operationId)',
            CloudOperation::class,
            FenceChain::class,
        ))->setParameter('library', $library->getId(), 'uuid')->setParameter('device', $deviceId)->setParameter('until', $until)
            ->setParameter('through', $through ?? new NilUuid(), 'uuid')->setParameter('fence', $fenceOperation, 'uuid')->setMaxResults(1)->getResult();

        return [] !== $found;
    }
}
