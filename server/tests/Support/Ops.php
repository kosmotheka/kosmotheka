<?php

declare(strict_types=1);

namespace App\Tests\Support;

use App\Dto\ChangePayload;
use App\Dto\OperationPayload;
use App\Enum\CatalogEntity;
use App\Enum\ChangeAction;
use Symfony\Component\Uid\Uuid;

/** Builds operations for the merge tests, already in the shape the Serializer would give. */
final class Ops
{
    /** The fixed device of the made-up operations; the HLC text is what the merge compares. */
    private const string DEVICE = '01997c7e-0000-7000-8000-0000000000fa';

    public static function hlc(int $time, int $counter = 0, string $device = self::DEVICE): string
    {
        return \sprintf('%016d-%08x-%s', $time, $counter, $device);
    }

    /**
     * @param list<ChangePayload> $changes
     */
    public static function operation(string $hlc, array $changes, string $library = 'library-local-id', ?string $signature = null): OperationPayload
    {
        return new OperationPayload(Uuid::v7()->toRfc4122(), $library, $hlc, 'test', $changes, null, $signature);
    }

    /**
     * @param array<string, mixed> $fields
     */
    public static function create(CatalogEntity $entity, string $id, array $fields): ChangePayload
    {
        return new ChangePayload($entity, $id, ChangeAction::Create, $fields);
    }

    /**
     * @param array<string, mixed> $fields
     */
    public static function update(CatalogEntity $entity, string $id, array $fields): ChangePayload
    {
        return new ChangePayload($entity, $id, ChangeAction::Update, $fields);
    }

    public static function delete(CatalogEntity $entity, string $id): ChangePayload
    {
        return new ChangePayload($entity, $id, ChangeAction::Delete);
    }
}
