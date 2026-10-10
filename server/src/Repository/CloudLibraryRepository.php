<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\CloudLibrary;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\Persistence\ManagerRegistry;

/**
 * @extends ServiceEntityRepository<CloudLibrary>
 */
final class CloudLibraryRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, CloudLibrary::class);
    }
}
