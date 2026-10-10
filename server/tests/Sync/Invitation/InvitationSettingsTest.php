<?php

declare(strict_types=1);

namespace App\Tests\Sync\Invitation;

use App\Sync\Invitation\InvitationSettings;
use PHPUnit\Framework\TestCase;

/** What `GET /v1/server` publishes as `invitation_base` comes from here (D43, D79). */
final class InvitationSettingsTest extends TestCase
{
    public function testAServerWithoutAnAddressForItsLinksMakesNoInvitationsAndPublishesNoBase(): void
    {
        $settings = new InvitationSettings('');

        self::assertFalse($settings->isEnabled());
        self::assertNull($settings->baseUrl());
    }

    public function testTheBaseIsTheAddressWithoutTheFinalSlashAndLinksStartWithIt(): void
    {
        $settings = new InvitationSettings('https://kosmotheka.app/');

        self::assertTrue($settings->isEnabled());
        self::assertSame('https://kosmotheka.app', $settings->baseUrl());
        self::assertStringStartsWith($settings->baseUrl().'/i/', $settings->linkFor('TOKEN'));
    }
}
