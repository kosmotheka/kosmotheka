<?php

declare(strict_types=1);

namespace App\Sync;

use App\Entity\Catalog\Member;
use App\Entity\CloudLibrary;
use App\Entity\Membership;
use App\Entity\User;
use App\Enum\ActivityKind;
use App\Security\ActivityLog;
use Doctrine\DBAL\LockMode;
use Doctrine\ORM\EntityManagerInterface;
use Doctrine\ORM\Query;

/**
 * What happens to the cloud libraries of an account that is deleted (D99). A library is not lost with the account of
 * whoever uploaded it: it passes to another account that has an active place in it (its membership not withdrawn and
 * its person without `access_withdrawn`, D98), an administrator's first, and among equals the oldest place, so the
 * result does not depend on the order the database gives rows in. A library nobody else can reach goes with the
 * account. The owner counts only for the first link and for this; everybody's access, hers included, is a membership.
 */
final readonly class CloudOwnership
{
    public function __construct(
        private EntityManagerInterface $entityManager,
        private ActivityLog $activityLog,
    ) {
    }

    /**
     * What deleting the account would do, without doing it: each library it owns, and the account that would receive it
     * (null: nobody else has a place, so it would go with the account). It asks the same way `handOverLibrariesOf` does,
     * but without the lock, so it is for showing a person, not for deciding.
     *
     * @return list<array{library: CloudLibrary, to: ?User}>
     */
    public function previewFor(User $leaving): array
    {
        $plan = [];
        foreach ($this->entityManager->getRepository(CloudLibrary::class)->findBy(['owner' => $leaving], ['createdAt' => 'ASC']) as $library) {
            $plan[] = ['library' => $library, 'to' => $this->successorOf($library, $leaving)?->getUser()];
        }

        return $plan;
    }

    /**
     * Inside the transaction of the deletion, under the lock of each library, as every write that changes who has it.
     * Nothing is flushed here: the caller's flush writes this and the deletion together.
     */
    public function handOverLibrariesOf(User $leaving): void
    {
        $libraries = $this->entityManager->getRepository(CloudLibrary::class)->findBy(['owner' => $leaving], ['createdAt' => 'ASC']);
        foreach ($libraries as $library) {
            $this->entityManager->refresh($library, LockMode::PESSIMISTIC_WRITE);
            $successor = $this->successorOf($library, $leaving);
            if (!$successor instanceof Membership) {
                // Explicitly: the database no longer deletes a library along with its owner (RESTRICT).
                $this->entityManager->remove($library);

                continue;
            }
            $library->passTo($successor->getUser());
            $this->activityLog->record(ActivityKind::LibraryOwnerChanged, $leaving, null, $library, $successor->getUser()->getId()->toRfc4122());
        }
    }

    /** The place that receives the library, read after the lock, from the server's own tables. */
    private function successorOf(CloudLibrary $library, User $leaving): ?Membership
    {
        /** @var list<Membership> $places */
        $places = $this->entityManager->createQuery(\sprintf('SELECT m FROM %s m WHERE m.cloudLibrary = :library AND m.withdrawnAt IS NULL AND m.user != :leaving', Membership::class))
            ->setParameter('library', $library->getId(), 'uuid')
            ->setParameter('leaving', $leaving->getId(), 'uuid')
            ->setHint(Query::HINT_REFRESH, true)
            ->getResult();

        /** @var list<array{id: string, role: ?string, withdrawn: ?bool}> $rows */
        $rows = $this->entityManager->createQuery(\sprintf('SELECT p.id AS id, p.role AS role, p.accessWithdrawn AS withdrawn FROM %s p WHERE p.cloudLibrary = :library', Member::class))
            ->setParameter('library', $library->getId(), 'uuid')
            ->getArrayResult();
        $people = [];
        foreach ($rows as $row) {
            $people[strtolower($row['id'])] = $row;
        }

        $candidates = [];
        foreach ($places as $place) {
            $person = $place->getMemberId()?->toRfc4122();
            $row = null === $person ? null : ($people[strtolower($person)] ?? null);
            if (true === ($row['withdrawn'] ?? null)) {
                continue;
            }
            $candidates[] = ['place' => $place, 'admin' => 'admin' === ($row['role'] ?? null)];
        }
        usort($candidates, static fn (array $a, array $b): int => [$b['admin'], $a['place']->getCreatedAt(), $a['place']->getId()->toRfc4122()]
            <=> [$a['admin'], $b['place']->getCreatedAt(), $b['place']->getId()->toRfc4122()]);

        return $candidates[0]['place'] ?? null;
    }
}
