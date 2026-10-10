<?php

declare(strict_types=1);

namespace App\Tests\Sync;

use App\Sync\OperationSignature;
use App\Sync\OperationSigner;
use App\Tests\Support\Spec;
use PHPUnit\Framework\TestCase;

/**
 * What the cloud signs is what the app accepts: Ed25519 is deterministic, so signing the operations of
 * spec/conformance/signatures.json with their test key (seed 32 × 0x01, as the Rust side signs them) gives
 * exactly their signatures. The app verifies the same bytes in its own tests.
 */
final class OperationSignerTest extends TestCase
{
    public function testItSignsTheConformanceOperationsWithTheSignaturesTheSpecHolds(): void
    {
        $secret = sodium_crypto_sign_secretkey(sodium_crypto_sign_seed_keypair(str_repeat("\x01", 32)));
        $signer = new OperationSigner(new OperationSignature());
        $checked = 0;

        foreach (Spec::json('conformance/signatures.json') as $case) {
            \assert(\is_array($case) && \is_array($case['operation'] ?? null) && \is_array($case['expect'] ?? null) && \is_string($case['name'] ?? null));
            if (($case['expect']['result'] ?? null) !== 'valid') {
                continue;
            }
            /** @var array<string, mixed> $operation */
            $operation = $case['operation'];
            $signature = $operation['signature'] ?? null;
            \assert(\is_string($signature));
            unset($operation['signature']);
            // The case lists the operation as decoded with arrays; empty objects would not survive that, so only those without any.
            if (str_contains(json_encode($operation, \JSON_THROW_ON_ERROR), '[]')) {
                continue;
            }

            self::assertSame($signature, $signer->sign($operation, $secret), $case['name']);
            ++$checked;
        }
        self::assertGreaterThan(0, $checked);
    }
}
