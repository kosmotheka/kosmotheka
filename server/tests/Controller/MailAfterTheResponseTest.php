<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpKernel\TerminableInterface;

/**
 * D100: the email of a code goes out after the response, in `kernel.terminate` (with PHP-FPM the response has already
 * left by then), so how long the answer takes does not tell whether an address has an account or is on the list.
 */
final class MailAfterTheResponseTest extends KernelTestCase
{
    use MailerAssertionsTrait;

    public function testTheCodeIsMailedOnlyOnceTheResponseIsOut(): void
    {
        self::bootKernel();
        self::getContainer()->get('cache.rate_limiter')->clear();
        $kernel = self::$kernel;
        self::assertInstanceOf(TerminableInterface::class, $kernel);

        $request = Request::create('/v1/account/register', 'POST', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode([
            'email' => 'later@example.com',
            'password' => 'correct horse battery',
            'name' => 'Ada',
            'locale' => 'en',
        ], \JSON_THROW_ON_ERROR));
        $response = $kernel->handle($request);

        self::assertSame(202, $response->getStatusCode());
        self::assertCount(0, self::getMailerMessages(), 'The email went out before the response.');

        $kernel->terminate($request, $response);

        self::assertCount(1, self::getMailerMessages());
    }
}
