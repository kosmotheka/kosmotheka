<?php

declare(strict_types=1);

namespace App\Sync;

use App\Entity\CloudLibrary;
use App\Entity\Membership;
use App\Entity\User;
use App\Repository\CloudLibraryRepository;
use App\Security\AccountException;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\DependencyInjection\Attribute\Autowire;

/**
 * Creates a cloud library for an account, with the id the server chooses, its cloud device (D95, if the
 * server has a secret) and the owner's membership. Until milestone 7.5 that is the only membership. An account owns
 * at most `$maxPerAccount` of them (D106): the libraries it is only a member of do not count.
 */
final readonly class CloudLibraryCreator
{
    public function __construct(
        private EntityManagerInterface $entityManager,
        private SyncNotifier $notifier,
        private CloudIdentityIssuer $identity,
        private CloudLibraryRepository $libraries,
        #[Autowire(param: 'app.cloud_max_libraries_per_account')]
        private int $maxPerAccount,
    ) {
    }

    /**
     * @throws AccountException when the account already owns as many as allowed
     */
    public function create(User $owner): CloudLibrary
    {
        if ($this->libraries->countOwnedBy($owner) >= $this->maxPerAccount) {
            throw AccountException::libraryLimit();
        }
        $library = new CloudLibrary($owner);
        $this->entityManager->persist($library);
        $this->identity->issueForNew($library);
        $this->entityManager->persist(new Membership($owner, $library));
        $this->entityManager->flush();
        $this->notifier->librariesOfAccountChanged($owner);

        return $library;
    }
}
