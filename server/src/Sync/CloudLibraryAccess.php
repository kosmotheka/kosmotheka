<?php

declare(strict_types=1);

namespace App\Sync;

use App\Entity\CloudLibrary;
use App\Entity\User;
use App\Repository\CloudLibraryRepository;
use App\Security\AccountException;
use Symfony\Component\Uid\Uuid;

/**
 * The door of every `{id}` cloud route (D84): the account must have a
 * membership in that cloud library. An id that is malformed, that does not
 * exist or that belongs to somebody else gets the same answer, so nobody can
 * learn which ids exist.
 */
final readonly class CloudLibraryAccess
{
    public function __construct(private CloudLibraryRepository $libraries)
    {
    }

    /**
     * @throws AccountException not found, for any of the three reasons above
     */
    public function require(string $id, User $user): CloudLibrary
    {
        if (!Uuid::isValid($id)) {
            throw AccountException::notFound();
        }

        return $this->libraries->findOneAccessibleBy(Uuid::fromString($id), $user) ?? throw AccountException::notFound();
    }
}
