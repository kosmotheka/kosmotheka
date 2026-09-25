<?php

declare(strict_types=1);

namespace App\Tests\Operation;

use App\Operation\Entity;
use App\Operation\MalformedOperation;
use App\Operation\OperationValidator;
use App\Tests\Spec;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

final class OperationValidatorTest extends TestCase
{
    /**
     * The cases every implementation of the spec shares with the Rust client
     * (crates/domain/src/operation.rs).
     *
     * @return iterable<string, array{array<mixed>, array<string, mixed>}>
     */
    public static function conformanceCases(): iterable
    {
        foreach (Spec::json('conformance/operation-validation.json') as $case) {
            \assert(\is_array($case) && \is_string($case['name']) && \is_array($case['operation']) && \is_array($case['expect']));

            yield $case['name'] => [$case['operation'], $case['expect']];
        }
    }

    /**
     * @param array<mixed>         $operation
     * @param array<string, mixed> $expect
     */
    #[DataProvider('conformanceCases')]
    public function testConformance(array $operation, array $expect): void
    {
        $error = new OperationValidator()->validate($operation);

        if ('valid' === $expect['result']) {
            self::assertNull($error, \sprintf('unexpected %s', $error?->code->value));

            return;
        }
        self::assertNotNull($error);
        self::assertSame(
            [$expect['code'], $expect['change'] ?? null, $expect['field'] ?? null],
            [$error->code->value, $error->change, $error->field],
        );
    }

    public function testCopyFormatsMatchTheSchema(): void
    {
        $schema = Spec::json('operation.schema.json');
        $json = json_encode($schema, \JSON_THROW_ON_ERROR);

        // The only enum in the schema with "hardcover" is copy.format.
        self::assertMatchesRegularExpression(
            '/"enum":\['.preg_quote(implode(',', array_map(json_encode(...), Entity::FORMATS)), '/').'\]/',
            $json,
        );
    }

    /**
     * @return iterable<string, array{array<mixed>}>
     */
    public static function malformedOperations(): iterable
    {
        $valid = [
            'id' => '01997c7e-0000-7000-8000-000000000001',
            'hlc' => '0001727222222222-00000000-dev',
            'intent' => 'member.add',
            'changes' => [[
                'entity' => 'member',
                'id' => '01997c7e-0000-7000-8000-000000000002',
                'action' => 'create',
                'fields' => ['name' => 'Tersoal'],
            ]],
        ];
        $withChange = static fn (array $patch): array => ['changes' => [array_merge($valid['changes'][0], $patch)]] + $valid;

        yield 'missing hlc' => [array_diff_key($valid, ['hlc' => true])];
        yield 'bad hlc' => [['hlc' => 'yesterday'] + $valid];
        yield 'unknown top-level key' => [$valid + ['author' => 'x']];
        yield 'changes is an object' => [['changes' => ['a' => 1]] + $valid];
        yield 'unknown entity' => [$withChange(['entity' => 'boardgame'])];
        yield 'unknown action' => [$withChange(['action' => 'upsert'])];
        yield 'fields is a list' => [$withChange(['fields' => ['Tersoal']])];
    }

    /**
     * @param array<mixed> $operation
     */
    #[DataProvider('malformedOperations')]
    public function testRejectsPayloadsThatAreNotOperations(array $operation): void
    {
        $this->expectException(MalformedOperation::class);

        new OperationValidator()->validate($operation);
    }
}
