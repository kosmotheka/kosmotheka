<?php

declare(strict_types=1);

namespace App\Tests\Validator;

use App\Dto\OperationPayload;
use App\Enum\CatalogEntity;
use App\Tests\Support\Spec;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;
use Symfony\Component\Serializer\Exception\ExceptionInterface;
use Symfony\Component\Serializer\SerializerInterface;
use Symfony\Component\Validator\ConstraintViolationListInterface;
use Symfony\Component\Validator\Validator\ValidatorInterface;

/**
 * Goes through the Serializer and the Validator, the same path a request takes
 * with #[MapRequestPayload].
 */
final class ValidOperationValidatorTest extends KernelTestCase
{
    /**
     * The cases every implementation of the spec shares with the Rust client
     * (crates/domain/src/operation.rs).
     *
     * @return iterable<string, array{array<mixed>, array<mixed>}>
     */
    public static function conformanceCases(): iterable
    {
        foreach (Spec::json('conformance/operation-validation.json') as $case) {
            \assert(\is_array($case) && \is_string($case['name']) && \is_array($case['operation']) && \is_array($case['expect']));

            yield $case['name'] => [$case['operation'], $case['expect']];
        }
    }

    /**
     * @param array<mixed> $operation
     * @param array<mixed> $expect
     */
    #[DataProvider('conformanceCases')]
    public function testConformance(array $operation, array $expect): void
    {
        $violations = $this->validate($operation);

        if ('valid' === $expect['result']) {
            self::assertCount(0, $violations, (string) $violations);

            return;
        }
        self::assertCount(1, $violations, (string) $violations);
        self::assertSame($expect['code'], $violations->get(0)->getCode());
        self::assertSame($this->expectedPath($expect), $violations->get(0)->getPropertyPath());
    }

    public function testCopyFormatsMatchTheSchema(): void
    {
        $json = json_encode(Spec::json('operation.schema.json'), \JSON_THROW_ON_ERROR);

        // The only enum in the schema with "hardcover" is copy.format.
        self::assertStringContainsString(
            '"enum":'.json_encode(CatalogEntity::FORMATS, \JSON_THROW_ON_ERROR),
            $json,
        );
    }

    public function testGenresMatchTheSchema(): void
    {
        $json = json_encode(Spec::json('operation.schema.json'), \JSON_THROW_ON_ERROR);

        self::assertStringContainsString(
            '"enum":'.json_encode(CatalogEntity::GENRES, \JSON_THROW_ON_ERROR),
            $json,
        );
    }

    /**
     * @return iterable<string, array{array<mixed>}>
     */
    public static function malformedOperations(): iterable
    {
        $valid = self::validOperation();
        $withChange = static fn (array $patch): array => ['changes' => [array_merge($valid['changes'][0], $patch)]] + $valid;

        yield 'missing hlc' => [array_diff_key($valid, ['hlc' => true])];
        yield 'missing library' => [array_diff_key($valid, ['library' => true])];
        yield 'unknown top-level key' => [$valid + ['author' => 'x']];
        yield 'intent is not a string' => [['intent' => 7] + $valid];
        yield 'unknown entity' => [$withChange(['entity' => 'boardgame'])];
        yield 'unknown action' => [$withChange(['action' => 'upsert'])];
        yield 'unknown change key' => [$withChange(['author' => 'x'])];
    }

    /**
     * @param array<mixed> $operation
     */
    #[DataProvider('malformedOperations')]
    public function testTheSerializerRejectsPayloadsThatAreNotOperations(array $operation): void
    {
        $this->expectException(ExceptionInterface::class);

        $this->validate($operation);
    }

    public function testRejectsAMalformedClock(): void
    {
        $violations = $this->validate(['hlc' => 'yesterday'] + self::validOperation());

        self::assertCount(1, $violations);
        self::assertSame('hlc', $violations->get(0)->getPropertyPath());
    }

    /**
     * @param array<mixed> $operation
     */
    private function validate(array $operation): ConstraintViolationListInterface
    {
        $container = self::getContainer();
        $serializer = $container->get(SerializerInterface::class);
        $validator = $container->get(ValidatorInterface::class);

        $payload = $serializer->deserialize(
            json_encode($operation, \JSON_THROW_ON_ERROR),
            OperationPayload::class,
            'json',
            OperationPayload::SERIALIZATION_CONTEXT,
        );

        return $validator->validate($payload);
    }

    /**
     * @param array<mixed> $expect
     */
    private function expectedPath(array $expect): string
    {
        $change = $expect['change'] ?? null;
        $field = $expect['field'] ?? null;
        if (null === $change) {
            return match ($expect['code']) {
                'invalid_id' => 'id',
                'invalid_library' => 'library',
                'invalid_intent' => 'intent',
                default => 'changes',
            };
        }
        \assert(\is_int($change));
        if (\is_string($field)) {
            return "changes[$change].fields[$field]";
        }

        return \in_array($expect['code'], ['invalid_id', 'other_library', 'other_device', 'wrong_id', 'undeletable'], true) ? "changes[$change].id" : "changes[$change].fields";
    }

    /**
     * @return array{id: string, library: string, hlc: string, intent: string, changes: list<array<string, mixed>>}
     */
    private static function validOperation(): array
    {
        return [
            'id' => '01997c7e-0000-7000-8000-000000000001',
            'library' => '01997c7e-0000-7000-8000-00000000aaaa',
            'hlc' => '0001727222222222-00000000-dev',
            'intent' => 'member.add',
            'changes' => [[
                'entity' => 'member',
                'id' => '01997c7e-0000-7000-8000-000000000002',
                'action' => 'create',
                'fields' => ['name' => 'Tersoal'],
            ]],
        ];
    }
}
