<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\HttpFoundation\Response;

/** `GET /v1/server` (D77): public, tells the app what this server offers. */
final class ServerControllerTest extends WebTestCase
{
    public function testShowReturnsWhatTheServerOffers(): void
    {
        $client = self::createClient();

        $client->request('GET', '/v1/server');

        self::assertResponseStatusCodeSame(Response::HTTP_OK);
        self::assertSame([
            'name' => 'Kosmotheka',
            'api_version' => 1,
            'registration' => true,
            'registration_restricted' => false,
            'sign_in_with_google' => false,
            'two_factor' => true,
            'events' => true,
            'invitations' => true,
            'invitation_base' => 'https://invitations.test',
        ], json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR));
    }
}
