<?php

declare(strict_types=1);

namespace App\Tests\Sync;

use App\Sync\CloudDeviceId;
use PHPUnit\Framework\TestCase;

/** The id of the cloud device derives from its key (D95): the same vector as `the_id_comes_from_the_key` in crates/domain/src/cloud.rs. */
final class CloudDeviceIdTest extends TestCase
{
    public function testTheIdComesFromTheKey(): void
    {
        self::assertSame('34750f98-bd59-8cfc-946d-a45aaabe933b', CloudDeviceId::fromPublicKey('iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w'));
        self::assertNull(CloudDeviceId::fromPublicKey('short'));
        self::assertNull(CloudDeviceId::fromPublicKey('not base64 !'));
    }
}
