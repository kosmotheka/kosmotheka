<?php

declare(strict_types=1);

namespace App\Tests\Sync\Merge;

use App\Dto\FieldSpec;
use App\Entity\Catalog\CatalogRow;
use App\Entity\CloudLibrary;
use App\Enum\CatalogEntity;
use App\Enum\FieldType;
use App\Sync\Merge\CatalogMerge;
use App\Sync\OperationReader;
use App\Tests\Support\CatalogRows;
use App\Tests\Support\Spec;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;

/**
 * The merge cases of spec/conformance, delivered to the engine in every order they list. Each operation
 * goes in alone, as in its own push, either all in memory or flushed after each one, so both the
 * merge's memory and the database are what the next change reads.
 *
 * @phpstan-type SpecRow array{library: string, entity: string, id: string, fields: array<string, mixed>}
 * @phpstan-type SpecCase array{name: string, operations: list<array{library: string, ...<string, mixed>}>, deliveries: list<list<int>>, expect: array{rows: list<SpecRow>, rejected?: list<string>}}
 */
final class MergeConformanceTest extends KernelTestCase
{
    /**
     * Cases about operations that are refused before they are stored. That is CloudPush's business and
     * not the merge's; they are covered when the merge is plugged into the push (7.4.6).
     */
    private const array NOT_THE_MERGES = [
        'two operations with one HLC: the second is refused and so are the later ones of its author' => 'The refusal of a second operation with the same HLC happens in CloudPush, before the merge (7.4.6).',
    ];

    private EntityManagerInterface $em;

    protected function setUp(): void
    {
        self::bootKernel();
        $this->em = self::getContainer()->get(EntityManagerInterface::class);
    }

    /**
     * @return iterable<string, array{string, string, int, bool}>
     */
    public static function deliveries(): iterable
    {
        foreach (['merge.json', 'library-merge.json'] as $file) {
            foreach (self::cases($file) as $case) {
                if (isset(self::NOT_THE_MERGES[$case['name']])) {
                    continue;
                }
                foreach (array_keys($case['deliveries']) as $order) {
                    foreach ([false, true] as $flushEach) {
                        yield \sprintf('%s: %s [order %d, %s]', $file, $case['name'], $order, $flushEach ? 'flush each' : 'in memory') => [$file, $case['name'], $order, $flushEach];
                    }
                }
            }
        }
    }

    #[DataProvider('deliveries')]
    public function testTheCaseEndsInTheExpectedRows(string $file, string $name, int $order, bool $flushEach): void
    {
        $case = $this->caseNamed($file, $name);
        $merge = self::getContainer()->get(CatalogMerge::class);
        $reader = self::getContainer()->get(OperationReader::class);
        $rows = new CatalogRows($this->em);

        /** @var array<string, CloudLibrary> $libraries by the library id of the spec */
        $libraries = [];
        foreach ($case['operations'] as $operation) {
            $libraries[$operation['library']] ??= $rows->cloudLibrary();
        }
        $this->em->flush();

        foreach ($case['deliveries'][$order] as $index) {
            $operation = json_decode((string) json_encode($case['operations'][$index]), false, flags: \JSON_THROW_ON_ERROR);
            \assert($operation instanceof \stdClass);
            $payload = $reader->read($operation);
            $merge->apply($libraries[$payload->library], $payload);
            if ($flushEach) {
                $merge->flush();
            }
            self::assertTrue($this->em->isOpen());
        }
        $merge->flush();
        self::assertTrue($this->em->isOpen());

        self::assertEquals(
            $this->sorted($case['expect']['rows']),
            $this->sorted($this->rowsInTheSpecShape(array_map(static fn (CloudLibrary $library): string => $library->getId()->toRfc4122(), $libraries))),
        );
    }

    public function testTheSkippedCasesAreExactlyThoseThatListRefusedOperations(): void
    {
        $refusing = [];
        foreach (['merge.json', 'library-merge.json'] as $file) {
            foreach (self::cases($file) as $case) {
                if ([] !== ($case['expect']['rejected'] ?? [])) {
                    $refusing[] = $case['name'];
                }
            }
        }
        $skipped = array_keys(self::NOT_THE_MERGES);
        sort($refusing);
        sort($skipped);

        self::assertSame($refusing, $skipped);
    }

    /**
     * Every row of the catalog, in any library: a row nobody expected is as wrong as a missing one.
     *
     * @param array<string> $cloudIds spec library id => cloud library id
     *
     * @return list<SpecRow>
     */
    private function rowsInTheSpecShape(array $cloudIds): array
    {
        $specLibrary = array_flip($cloudIds);
        $found = [];
        foreach (CatalogEntity::cases() as $entity) {
            $metadata = $this->em->getClassMetadata($entity->entityClass());
            foreach ($this->em->getRepository($entity->entityClass())->findAll() as $row) {
                $fields = [];
                foreach ($entity->fields() as $key => $spec) {
                    $property = $this->property($key, $spec);
                    if (!$metadata->hasField($property) && !$metadata->hasAssociation($property)) {
                        continue; // library.cloud_library_id and cloud_server have no column
                    }
                    $value = $metadata->getFieldValue($row, $property);
                    $value = $value instanceof CatalogRow ? $value->getId() : $value;
                    if (null !== $value) {
                        $fields[$key] = $value;
                    }
                }
                $found[] = [
                    'library' => $specLibrary[$row->getCloudLibrary()->getId()->toRfc4122()] ?? 'unexpected library',
                    'entity' => $entity->value,
                    'id' => $row->getId(),
                    'fields' => $fields,
                ];
            }
        }

        return $found;
    }

    private function property(string $key, FieldSpec $spec): string
    {
        $base = FieldType::Ref === $spec->type ? substr($key, 0, -3) : $key;

        return lcfirst(str_replace('_', '', ucwords($base, '_')));
    }

    /**
     * @param list<SpecRow> $rows
     *
     * @return list<SpecRow>
     */
    private function sorted(array $rows): array
    {
        usort($rows, static fn (array $a, array $b): int => [$a['library'], $a['entity'], $a['id']] <=> [$b['library'], $b['entity'], $b['id']]);

        return array_map(static function (array $row): array {
            ksort($row['fields']);

            return $row;
        }, $rows);
    }

    /**
     * @return list<SpecCase>
     */
    private static function cases(string $file): array
    {
        /** @var list<SpecCase> $cases */
        $cases = Spec::json('conformance/'.$file);

        return $cases;
    }

    /**
     * @return SpecCase
     */
    private function caseNamed(string $file, string $name): array
    {
        foreach (self::cases($file) as $case) {
            if ($case['name'] === $name) {
                return $case;
            }
        }
        self::fail('No case named '.$name);
    }
}
