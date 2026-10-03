<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\CloudLibrary;
use App\Entity\CloudOperation;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\DBAL\ArrayParameterType;
use Doctrine\Persistence\ManagerRegistry;

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

    /**
     * Stores the operation as received, unless the library's log already has it.
     *
     * @return bool whether it was stored (false: it was already known)
     */
    public function insertIfNew(CloudLibrary $library, string $operationId, string $authorDeviceId, string $hlc, int $schemaVersion, string $bodyJson): bool
    {
        $inserted = $this->getEntityManager()->getConnection()->executeStatement(
            'INSERT INTO cloud_operation (cloud_library_id, operation_id, author_device_id, hlc, schema_version, body, received_at)
             VALUES (:library, :operation, :author, :hlc, :schema, CAST(:body AS JSONB), :received)
             ON CONFLICT (cloud_library_id, operation_id) DO NOTHING',
            [
                'library' => $library->getId()->toRfc4122(),
                'operation' => $operationId,
                'author' => $authorDeviceId,
                'hlc' => $hlc,
                'schema' => $schemaVersion,
                'body' => $bodyJson,
                'received' => new \DateTimeImmutable('now', new \DateTimeZone('UTC'))->format('Y-m-d H:i:s'),
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
}
