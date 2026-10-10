<?php

declare(strict_types=1);

namespace App\Sync;

use App\Entity\CloudLibrary;
use App\Entity\Membership;
use App\Entity\User;
use Doctrine\ORM\EntityManagerInterface;

/**
 * Creates a cloud library for an account, with the id the server chooses, its cloud device (D95, if the
 * server has a secret) and the owner's membership. Until milestone 7.5 that is the only membership.
 */
final readonly class CloudLibraryCreator
{
    public function __construct(private EntityManagerInterface $entityManager, private SyncNotifier $notifier, private CloudIdentityIssuer $identity)
    {
    }

    public function create(User $owner): CloudLibrary
    {
        $library = new CloudLibrary($owner);
        $this->entityManager->persist($library);
        $this->identity->issueForNew($library);
        $this->entityManager->persist(new Membership($owner, $library));
        $this->entityManager->flush();
        $this->notifier->librariesOfAccountChanged($owner);

        return $library;
    }
}
