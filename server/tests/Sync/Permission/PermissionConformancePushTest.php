<?php

declare(strict_types=1);

namespace App\Tests\Sync\Permission;

use App\Dto\PushedOperation;
use App\Dto\PushRequest;
use App\Dto\PushResult;
use App\Entity\CloudLibrary;
use App\Entity\Device;
use App\Enum\OperationStatus;
use App\Sync\CloudPush;
use App\Sync\OperationSignature;
use App\Sync\SchemaVersion;
use App\Tests\Support\CatalogRows;
use App\Tests\Support\Spec;
use Doctrine\DBAL\Connection;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;

/**
 * The cases of spec/conformance/permissions.json through the whole push (7.4.6): signature, key, chain and judge.
 * Each operation is pushed alone in the order the delivery lists; one the cloud refuses because the operation it
 * follows has not arrived is sent again after the others, as the app does in its next sync (D87). Unlike the
 * judge's own test, the operations a case expects `rejected` are pushed too, and must be refused.
 *
 * @phpstan-type PermissionCase array{name: string, operations: list<array<string, mixed>>, deliveries: list<list<int>>, expect: array{applied: list<string>, requests: list<string>, rejected?: list<string>}}
 */
final class PermissionConformancePushTest extends KernelTestCase
{
    /** The library id every case's operations carry. */
    private const string LOCAL = '01997c7e-0000-7000-8000-00000000aaaa';

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
    public function testTheCaseEndsInTheExpectedStatusesAndRefusals(string $name, int $order): void
    {
        $case = $this->caseNamed($name);
        [$library, $uploader] = $this->linkedLibrary();
        $expectedRejected = $case['expect']['rejected'] ?? [];

        $pending = $case['deliveries'][$order];
        $refused = [];
        while ([] !== $pending) {
            $waiting = [];
            foreach ($pending as $index) {
                $id = $case['operations'][$index]['id'];
                \assert(\is_string($id));
                $result = $this->push($library, $uploader, $this->signed($case['operations'][$index]));
                self::assertSame(1, $result->received + $result->known + $result->rejected);
                if (0 === $result->rejected) {
                    self::assertNotContains($id, $expectedRejected, 'stored, but the case expects it refused');
                } elseif (\in_array($id, $expectedRejected, true)) {
                    $refused[$id] = true;
                } else {
                    $waiting[] = $index;
                }
            }
            self::assertNotSame($pending, $waiting, 'Nothing more could be stored; still waiting: '.implode(', ', array_map(static fn (int $i): string => self::idOf($case['operations'][$i]), $waiting)));
            $pending = $waiting;
        }

        self::assertEqualsCanonicalizing($expectedRejected, array_keys($refused), 'the refused ones');
        self::assertSame(
            ['applied' => $this->sorted($case['expect']['applied']), 'request' => $this->sorted($case['expect']['requests'])],
            $this->statuses($library),
        );
        self::assertTrue($this->em->isOpen());
    }

    /** The case of merge.json that the merge engine alone cannot run: the refusal happens in the push. */
    public function testTwoOperationsWithOneHlcTheSecondIsRefusedAndSoAreTheLaterOnesOfItsAuthor(): void
    {
        /** @var list<array{name: string, operations: list<array<string, mixed>>, deliveries: list<list<int>>, expect: array{rows: list<array{id: string, fields: array<string, mixed>}>, rejected: list<string>}}> $cases */
        $cases = Spec::json('conformance/merge.json');
        $case = null;
        foreach ($cases as $candidate) {
            if (str_starts_with($candidate['name'], 'two operations with one HLC')) {
                $case = $candidate;
            }
        }
        self::assertNotNull($case);
        $hlc = $case['operations'][0]['hlc'];
        \assert(\is_string($hlc));
        $device = explode('-', $hlc, 3)[2];

        foreach ($case['deliveries'] as $order => $delivery) {
            [$library, $uploader] = $this->linkedLibrary();
            $registration = ['id' => '01997c7e-0000-7000-8000-0000000004ff', 'library' => self::LOCAL, 'hlc' => '0001727222222000-00000000-'.$device, 'intent' => 'device.register',
                'changes' => [['entity' => 'device', 'id' => $device, 'action' => 'create', 'fields' => ['public_key' => self::publicKey(), 'name' => 'laptop']]], 'previous' => null];
            self::assertSame(1, $this->push($library, $uploader, $this->signed($registration))->received);

            $results = [];
            foreach ($delivery as $index) {
                $results[self::idOf($case['operations'][$index])] = $this->push($library, $uploader, $this->signed($case['operations'][$index]));
            }

            foreach ($case['expect']['rejected'] as $id) {
                self::assertSame(1, $results[$id]->rejected, "order $order: $id is refused");
            }
            $stored = $this->storedIds($library);
            foreach ($case['expect']['rejected'] as $id) {
                self::assertNotContains($id, $stored, "order $order: $id is not in the log");
            }
            $works = $this->em->getConnection()->fetchAllAssociative('SELECT id, title FROM catalog_works WHERE cloud_library_id = ?', [$library->getId()->toRfc4122()]);
            self::assertSame([['id' => $case['expect']['rows'][0]['id'], 'title' => $case['expect']['rows'][0]['fields']['title']]], $works, "order $order");
            $this->em->clear();
        }
    }

    public function testTheCasesThatListRejectedOperationsAreExactlyTheExpectedOnes(): void
    {
        $listing = [];
        foreach (self::cases() as $case) {
            if ([] !== ($case['expect']['rejected'] ?? [])) {
                $listing[] = $case['name'];
            }
        }
        sort($listing);

        self::assertSame([
            'An unsigned operation is refused, with or without roles',
            'an author written in capitals is not read, so the cloud\'s id in capitals gets none of its power (SEG-029)',
            'the key of a device counts only through its own create, and a registered one never changes',
        ], $listing);
    }

    /**
     * @return array{CloudLibrary, Device} a cloud library linked to the cases' local library, and a device of its owner
     */
    private function linkedLibrary(): array
    {
        $library = new CatalogRows($this->em)->cloudLibrary();
        $library->link(self::LOCAL);
        $uploader = new Device(bin2hex(random_bytes(16)), $library->getOwner(), 'Uploader', self::publicKey());
        $this->em->persist($uploader);
        $this->em->flush();

        return [$library, $uploader];
    }

    private function push(CloudLibrary $library, Device $uploader, \stdClass $operation): PushResult
    {
        $version = self::getContainer()->get(SchemaVersion::class)->current();

        return self::getContainer()->get(CloudPush::class)->push($library, new PushRequest($version, [new PushedOperation($version, $operation)]), $uploader);
    }

    /**
     * Signed with the test key every device in the cases registers (spec/conformance/signatures.json): seed 32 × 0x01.
     * An operation the case marks `unsigned` goes out without one.
     *
     * @param array<string, mixed> $fields
     */
    private function signed(array $fields): \stdClass
    {
        $unsigned = isset($fields['unsigned']);
        unset($fields['unsigned']);
        $operation = json_decode((string) json_encode($fields), false, flags: \JSON_THROW_ON_ERROR);
        \assert($operation instanceof \stdClass);
        if (!$unsigned) {
            $secret = sodium_crypto_sign_secretkey(sodium_crypto_sign_seed_keypair(str_repeat("\x01", 32)));
            $operation->signature = sodium_bin2base64(sodium_crypto_sign_detached(new OperationSignature()->canonical($operation), $secret), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        }

        return $operation;
    }

    /**
     * @param array<string, mixed> $operation
     */
    private static function idOf(array $operation): string
    {
        $id = $operation['id'] ?? null;
        \assert(\is_string($id));

        return $id;
    }

    private function publicKey(): string
    {
        return sodium_bin2base64(sodium_crypto_sign_publickey(sodium_crypto_sign_seed_keypair(str_repeat("\x01", 32))), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
    }

    /**
     * @return list<string>
     */
    private function storedIds(CloudLibrary $library): array
    {
        return array_map(static fn (mixed $id): string => \is_string($id) ? $id : '', self::getContainer()->get(Connection::class)->fetchFirstColumn('SELECT operation_id FROM cloud_operation WHERE cloud_library_id = ?', [$library->getId()->toRfc4122()]));
    }

    /**
     * @return array{applied: list<string>, request: list<string>}
     */
    private function statuses(CloudLibrary $library): array
    {
        $found = ['applied' => [], 'request' => []];
        $rows = self::getContainer()->get(Connection::class)->fetchAllAssociative('SELECT operation_id, status FROM cloud_operation WHERE cloud_library_id = ?', [$library->getId()->toRfc4122()]);
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
