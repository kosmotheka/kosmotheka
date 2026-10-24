<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\Catalog\Edition;
use App\Entity\CloudCover;
use App\Entity\CloudLibrary;
use App\Sync\CloudCovers;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\Persistence\ManagerRegistry;

/**
 * @extends ServiceEntityRepository<CloudCover>
 */
final class CloudCoverRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, CloudCover::class);
    }

    /**
     * The names of the covers kept for a library.
     *
     * @return list<string>
     */
    public function namesIn(CloudLibrary $library): array
    {
        $names = $this->createQueryBuilder('c')
            ->select('c.name')
            ->where('c.cloudLibrary = :library')
            ->setParameter('library', $library)
            ->getQuery()
            ->getSingleColumnResult();

        return array_values(array_filter($names, \is_string(...)));
    }

    public function findIn(CloudLibrary $library, string $name): ?CloudCover
    {
        return $this->findOneBy(['cloudLibrary' => $library, 'name' => $name]);
    }

    /** What the covers of a library take, in bytes. */
    public function sizeOf(CloudLibrary $library): int
    {
        $total = $this->createQueryBuilder('c')
            ->select('COALESCE(SUM(c.size), 0)')
            ->where('c.cloudLibrary = :library')
            ->setParameter('library', $library)
            ->getQuery()
            ->getSingleScalarResult();

        return (int) $total;
    }

    /**
     * The cover names the editions of a library point to: the only covers it may keep.
     *
     * @return list<string>
     */
    public function referencedIn(CloudLibrary $library): array
    {
        $names = $this->getEntityManager()->createQueryBuilder()
            ->select('DISTINCT e.cover')
            ->from(Edition::class, 'e')
            ->where('e.cloudLibrary = :library')
            ->andWhere('e.cover IS NOT NULL')
            ->setParameter('library', $library)
            ->getQuery()
            ->getSingleColumnResult();

        // Only names a cover can have: the field is free text in an operation.
        return array_values(array_filter($names, static fn (mixed $name): bool => \is_string($name) && 1 === preg_match('/\A'.CloudCovers::NAME.'\z/', $name)));
    }

    public function isReferencedIn(CloudLibrary $library, string $name): bool
    {
        return null !== $this->getEntityManager()->createQueryBuilder()
            ->select('e.key')
            ->from(Edition::class, 'e')
            ->where('e.cloudLibrary = :library')
            ->andWhere('e.cover = :name')
            ->setParameter('library', $library)
            ->setParameter('name', $name)
            ->setMaxResults(1)
            ->getQuery()
            ->getOneOrNullResult();
    }

    /**
     * Covers no edition of their library points to any more, kept since before `$before`.
     *
     * @return list<CloudCover>
     */
    public function unreferencedBefore(\DateTimeImmutable $before): array
    {
        /* @var list<CloudCover> */
        return $this->createQueryBuilder('c')
            ->where('c.storedAt < :before')
            ->andWhere('NOT EXISTS (SELECT 1 FROM '.Edition::class.' e WHERE e.cloudLibrary = c.cloudLibrary AND e.cover = c.name)')
            ->setParameter('before', $before)
            ->getQuery()
            ->getResult();
    }
}
