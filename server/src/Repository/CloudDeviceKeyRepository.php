<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\CloudDeviceKey;
use App\Entity\CloudLibrary;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\Persistence\ManagerRegistry;

/**
 * @extends ServiceEntityRepository<CloudDeviceKey>
 */
final class CloudDeviceKeyRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, CloudDeviceKey::class);
    }

    /** Always inside one cloud library: a device id means nothing across them (D84). */
    public function findOneInLibrary(CloudLibrary $library, string $deviceId): ?CloudDeviceKey
    {
        return $this->findOneBy(['cloudLibrary' => $library, 'deviceId' => $deviceId]);
    }
}
