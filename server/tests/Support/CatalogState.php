<?php

declare(strict_types=1);

namespace App\Tests\Support;

use App\Entity\Catalog\CatalogRow;
use App\Entity\Catalog\FieldVersion;
use App\Entity\Catalog\Tombstone;
use App\Entity\CloudLibrary;
use App\Enum\CatalogEntity;
use App\Enum\FieldType;
use Doctrine\ORM\EntityManagerInterface;

/**
 * Everything the merge keeps for a cloud library, in a form two libraries can be compared by: the rows
 * with their references as app ids, the version of every field and the tombstones. It leaves nothing managed.
 */
final class CatalogState
{
    /**
     * @return array{rows: array<string, array<string, mixed>>, versions: array<string, string>, tombstones: array<string, string>}
     */
    public static function of(EntityManagerInterface $entityManager, CloudLibrary $library): array
    {
        $loaded = [];
        $rows = [];
        foreach (CatalogEntity::cases() as $entity) {
            $metadata = $entityManager->getClassMetadata($entity->entityClass());
            foreach ($entityManager->getRepository($entity->entityClass())->findBy(['cloudLibrary' => $library]) as $row) {
                $loaded[] = $row;
                $fields = [];
                foreach ($entity->fields() as $key => $spec) {
                    $base = FieldType::Ref === $spec->type ? substr($key, 0, -3) : $key;
                    $property = lcfirst(str_replace('_', '', ucwords($base, '_')));
                    if (!$metadata->hasField($property) && !$metadata->hasAssociation($property)) {
                        continue;
                    }
                    $value = $metadata->getFieldValue($row, $property);
                    $fields[$key] = $value instanceof CatalogRow ? $value->getId() : $value;
                }
                $rows[$entity->value.'|'.$row->getId()] = $fields;
            }
        }
        ksort($rows);

        $versions = [];
        foreach ($entityManager->getRepository(FieldVersion::class)->findBy(['cloudLibrary' => $library]) as $version) {
            $loaded[] = $version;
            $versions[$version->getEntity().'|'.$version->getEntityId().'|'.$version->getField()] = $version->getHlc();
        }
        ksort($versions);

        $tombstones = [];
        foreach ($entityManager->getRepository(Tombstone::class)->findBy(['cloudLibrary' => $library]) as $tombstone) {
            $loaded[] = $tombstone;
            $tombstones[$tombstone->getEntity().'|'.$tombstone->getEntityId()] = $tombstone->getHlc();
        }
        ksort($tombstones);

        // The merge detaches what it loads; whatever else is managed and points into it would break its next flush.
        foreach ($loaded as $object) {
            $entityManager->detach($object);
        }

        return ['rows' => $rows, 'versions' => $versions, 'tombstones' => $tombstones];
    }

    /**
     * @param array{rows: array<string, array<string, mixed>>, versions: array<string, string>, tombstones: array<string, string>} $state
     *
     * @return array<string, int> rows per entity
     */
    public static function countsPerEntity(array $state): array
    {
        $counts = [];
        foreach (array_keys($state['rows']) as $key) {
            $entity = explode('|', $key, 2)[0];
            $counts[$entity] = ($counts[$entity] ?? 0) + 1;
        }
        ksort($counts);

        return $counts;
    }
}
