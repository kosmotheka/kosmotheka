<?php

declare(strict_types=1);

namespace App\Tests\Sync;

use App\Sync\InvalidSignatureException;
use App\Sync\OperationSignature;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

/**
 * The signature cases every implementation shares with the Rust client
 * (crates/domain/src/signature.rs).
 */
final class OperationSignatureTest extends TestCase
{
    /**
     * @return iterable<string, array{\stdClass, string, string, \stdClass}>
     */
    public static function conformanceCases(): iterable
    {
        $contents = file_get_contents(\dirname(__DIR__, 3).'/spec/conformance/signatures.json');
        self::assertIsString($contents);
        // Objects as \stdClass, so the canonical form sees exactly what was sent.
        $cases = json_decode($contents, flags: \JSON_THROW_ON_ERROR);
        self::assertIsArray($cases);
        foreach ($cases as $case) {
            \assert($case instanceof \stdClass);

            yield $case->name => [$case->operation, $case->publicKey, $case->canonical, $case->expect];
        }
    }

    #[DataProvider('conformanceCases')]
    public function testConformance(\stdClass $operation, string $publicKey, string $canonical, \stdClass $expect): void
    {
        $signature = new OperationSignature();

        self::assertSame($canonical, $signature->canonical($operation));

        try {
            $signature->verify($operation, $publicKey);
            self::assertSame('valid', $expect->result);
        } catch (InvalidSignatureException $exception) {
            self::assertSame('invalid', $expect->result);
            self::assertSame($expect->code, $exception->reason->value);
        }
    }
}
