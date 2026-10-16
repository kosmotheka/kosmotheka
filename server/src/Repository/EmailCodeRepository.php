<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\EmailCode;
use App\Entity\User;
use App\Enum\EmailCodePurpose;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\ORM\AbstractQuery;
use Doctrine\Persistence\ManagerRegistry;

/**
 * @extends ServiceEntityRepository<EmailCode>
 */
final class EmailCodeRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, EmailCode::class);
    }

    public function findOneByUserAndPurpose(User $user, EmailCodePurpose $purpose): ?EmailCode
    {
        return $this->findOneBy(['user' => $user, 'purpose' => $purpose]);
    }

    /**
     * One more wrong attempt, added in the database: two at once both count (SEG-072).
     *
     * @return int|null the attempts now, or null when the code is gone already
     */
    public function countFailedAttempt(EmailCode $code): ?int
    {
        $this->getEntityManager()->createQuery(\sprintf('UPDATE %s c SET c.attempts = c.attempts + 1 WHERE c.id = :id', EmailCode::class))
            ->setParameter('id', $code->getId(), 'uuid')->execute();
        $attempts = $this->getEntityManager()->createQuery(\sprintf('SELECT c.attempts FROM %s c WHERE c.id = :id', EmailCode::class))
            ->setParameter('id', $code->getId(), 'uuid')->getOneOrNullResult(AbstractQuery::HYDRATE_SINGLE_SCALAR);

        return is_numeric($attempts) ? (int) $attempts : null;
    }

    /** Takes the code away and says whether it was still there: of two requests that spend it at once, one does (SEG-072). */
    public function delete(EmailCode $code): bool
    {
        $deleted = $this->getEntityManager()->createQuery(\sprintf('DELETE FROM %s c WHERE c.id = :id', EmailCode::class))
            ->setParameter('id', $code->getId(), 'uuid')->execute();
        $this->getEntityManager()->detach($code);

        return 1 === $deleted;
    }
}
