<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\Device;
use App\Entity\User;
use App\Enum\SignOutReason;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\Persistence\ManagerRegistry;

/**
 * @extends ServiceEntityRepository<Device>
 */
final class DeviceRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, Device::class);
    }

    /**
     * Signs out every device of the account, except one (the device making
     * the call, when there is one). Used after a password change or reset
     * (D50) and when blocking an account (D51). With a reason, the token hash
     * stays so the install can be told why on its next call; without one, it
     * is dropped. A bulk update, since nothing here needs the Device entities
     * back in memory afterwards.
     */
    public function signOutAllExcept(User $user, ?string $exceptDeviceId, ?SignOutReason $reason = null): void
    {
        $queryBuilder = $this->createQueryBuilder('d')
            ->update()
            ->where('d.user = :user')
            ->setParameter('user', $user);

        if ($reason instanceof SignOutReason) {
            $queryBuilder->set('d.signOutReason', ':reason')->setParameter('reason', $reason->value)
                ->andWhere('d.tokenHash IS NOT NULL');
        } else {
            $queryBuilder->set('d.tokenHash', ':tokenHash')->setParameter('tokenHash', null);
        }

        if (null !== $exceptDeviceId) {
            $queryBuilder->andWhere('d.id != :exceptDeviceId')->setParameter('exceptDeviceId', $exceptDeviceId);
        }

        $queryBuilder->getQuery()->execute();
    }
}
