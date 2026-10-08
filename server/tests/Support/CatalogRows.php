<?php

declare(strict_types=1);

namespace App\Tests\Support;

use App\Entity\Catalog\CatalogRow;
use App\Entity\CloudLibrary;
use App\Entity\User;
use Doctrine\ORM\EntityManagerInterface;

/**
 * Builds catalog rows for tests the way the merge engine will: the constructor
 * for the key, Doctrine's metadata for the fields (the entities have no setters).
 */
final readonly class CatalogRows
{
    public function __construct(private EntityManagerInterface $entityManager)
    {
    }

    /** A cloud library with its own owner, persisted but not flushed. */
    public function cloudLibrary(): CloudLibrary
    {
        $owner = new User(\sprintf('rows-%s@example.com', bin2hex(random_bytes(4))), 'Ada', 'en');
        $library = new CloudLibrary($owner);
        $this->entityManager->persist($owner);
        $this->entityManager->persist($library);

        return $library;
    }

    /**
     * @template T of CatalogRow
     *
     * @param class-string<T>      $class
     * @param array<string, mixed> $values property name => value; references are rows
     *
     * @return T
     */
    public function row(string $class, CloudLibrary $library, string $id, array $values = []): CatalogRow
    {
        $row = new $class($library, $id);
        $metadata = $this->entityManager->getClassMetadata($class);
        foreach ($values as $property => $value) {
            $metadata->setFieldValue($row, $property, $value);
        }
        $this->entityManager->persist($row);

        return $row;
    }
}
