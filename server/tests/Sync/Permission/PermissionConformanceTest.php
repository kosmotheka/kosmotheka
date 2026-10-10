<?php

declare(strict_types=1);

namespace App\Tests\Sync\Permission;

use App\Enum\OperationStatus;
use App\Repository\CloudOperationRepository;
use App\Sync\OperationReader;
use App\Sync\OperationSignature;
use App\Sync\Permission\CatalogJudge;
use App\Sync\SchemaVersion;
use App\Tests\Support\CatalogRows;
use App\Tests\Support\Spec;
use Doctrine\DBAL\Connection;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;

/**
 * The cases of spec/conformance/permissions.json, delivered to the judge in every order they list. Each
 * operation goes in alone, in its own batch (stored, taken, settled), as the Rust runner does with its own
 * `merge`. The operations a case expects `rejected` never reach the judge: refusing them (signature, key)
 * is CloudPush's business, and the whole push will check them in 7.4.6.
 *
 * @phpstan-type PermissionCase array{name: string, operations: list<array<string, mixed>>, deliveries: list<list<int>>, expect: array{applied: list<string>, requests: list<string>, rejected?: list<string>}}
 */
final class PermissionConformanceTest extends KernelTestCase
{
    /** The cases that list operations as rejected: a new one must not pass without somebody looking at it. */
    private const array CASES_WITH_REJECTED = [
        'An unsigned operation is refused, with or without roles',
        'the key of a device counts only through its own create, and a registered one never changes',
        'an author written in capitals is not read, so the cloud\'s id in capitals gets none of its power (SEG-029)',
    ];

    private EntityManagerInterface $em;

    protected function setUp(): void
    {
        self::bootKernel();
        $this->em = self::getContainer()->get(EntityManagerInterface::class);
    }

    /**
     * @return iterable<string, array{string, int}>
     */
    public static function deliveries(): iterable
    {
        foreach (self::cases() as $case) {
            foreach (array_keys($case['deliveries']) as $order) {
                yield \sprintf('%s [order %d]', $case['name'], $order) => [$case['name'], $order];
            }
        }
    }

    #[DataProvider('deliveries')]
    public function testTheCaseEndsInTheExpectedStatuses(string $name, int $order): void
    {
        $case = $this->caseNamed($name);
        $judge = self::getContainer()->get(CatalogJudge::class);
        $operations = self::getContainer()->get(CloudOperationRepository::class);
        $reader = self::getContainer()->get(OperationReader::class);
        $schema = self::getContainer()->get(SchemaVersion::class)->current();
        $signature = new OperationSignature();
        // The test key every device in the cases registers (spec/conformance/signatures.json): seed 32 × 0x01.
        $secret = sodium_crypto_sign_secretkey(sodium_crypto_sign_seed_keypair(str_repeat("\x01", 32)));

        $library = new CatalogRows($this->em)->cloudLibrary();
        $this->em->flush();
        $rejected = $case['expect']['rejected'] ?? [];

        foreach ($case['deliveries'][$order] as $index) {
            $fields = $case['operations'][$index];
            unset($fields['unsigned']);
            $id = $fields['id'];
            \assert(\is_string($id));
            if (\in_array($id, $rejected, true)) {
                continue;
            }
            $operation = json_decode((string) json_encode($fields), false, flags: \JSON_THROW_ON_ERROR);
            \assert($operation instanceof \stdClass);
            $operation->signature = sodium_bin2base64(sodium_crypto_sign_detached($signature->canonical($operation), $secret), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);

            $payload = $reader->read($operation);
            $stored = $operations->insertIfNew($library, $payload->id, explode('-', $payload->hlc, 3)[2], $payload->hlc, $schema, json_encode($operation, \JSON_THROW_ON_ERROR), $payload->previous);
            self::assertTrue($stored);
            $judge->take($library, $payload);
            $judge->settle($library);
            self::assertTrue($this->em->isOpen());
        }

        $expected = ['applied' => $this->sorted($case['expect']['applied']), 'request' => $this->sorted($case['expect']['requests'])];
        self::assertSame($expected, $this->statuses($library->getId()->toRfc4122()), 'as delivered');

        // The same verdicts when the whole log is judged again in HLC order (`assert_rebuild_changes_nothing`).
        $judge->rebuild($library);
        self::assertSame($expected, $this->statuses($library->getId()->toRfc4122()), 'after a rebuild');
    }

    public function testTheCasesThatListRejectedOperationsAreExactlyTheExpectedOnes(): void
    {
        $listing = [];
        foreach (self::cases() as $case) {
            if ([] !== ($case['expect']['rejected'] ?? [])) {
                $listing[] = $case['name'];
            }
        }
        $expected = self::CASES_WITH_REJECTED;
        sort($listing);
        sort($expected);

        self::assertSame($expected, $listing);
    }

    /**
     * @return array{applied: list<string>, request: list<string>}
     */
    private function statuses(string $cloudLibrary): array
    {
        $found = ['applied' => [], 'request' => []];
        $rows = self::getContainer()->get(Connection::class)->fetchAllAssociative('SELECT operation_id, status FROM cloud_operation WHERE cloud_library_id = :id', ['id' => $cloudLibrary]);
        foreach ($rows as $row) {
            \assert(\is_string($row['operation_id']));
            $status = OperationStatus::tryFrom(\is_string($row['status']) ? $row['status'] : '');
            self::assertNotNull($status, 'Judged: '.$row['operation_id']);
            $found[$status->value][] = $row['operation_id'];
        }

        return ['applied' => $this->sorted($found['applied']), 'request' => $this->sorted($found['request'])];
    }

    /**
     * @param list<string> $ids
     *
     * @return list<string>
     */
    private function sorted(array $ids): array
    {
        sort($ids);

        return $ids;
    }

    /**
     * @return list<PermissionCase>
     */
    private static function cases(): array
    {
        /** @var list<PermissionCase> $cases */
        $cases = Spec::json('conformance/permissions.json');

        return $cases;
    }

    /**
     * @return PermissionCase
     */
    private function caseNamed(string $name): array
    {
        foreach (self::cases() as $case) {
            if ($case['name'] === $name) {
                return $case;
            }
        }
        self::fail('No case named '.$name);
    }
}
