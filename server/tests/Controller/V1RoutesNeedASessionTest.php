<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\Routing\RouterInterface;

/**
 * `access_control` in `security.yaml` is the only place that asks for a session on `/v1` (the controllers carry no
 * `#[IsGranted]` for it): every route of the router under `/v1` that is not one of the public ones answers 401 without
 * one. The routes come from the router, so a new one is covered the day it is written.
 */
final class V1RoutesNeedASessionTest extends WebTestCase
{
    /** The `PUBLIC_ACCESS` lines of `security.yaml`, as a method and a path. */
    private const array PUBLIC = [
        '#^/v1/account/password/(forgot|reset)$#',
        '#^/v1/account/(register|confirm|code)$#',
        '#^/v1/auth/(challenge|login)$#',
        '#^/v1/auth/google/(start|connect|callback|finish)$#',
        '#^/v1/server$#',
    ];

    /** Looking at an invitation by its token needs no session; accepting one does. */
    private const string PUBLIC_GET = '#^/v1/cloud/invitations/[^/]+$#';

    /** @return iterable<string, array{string, string}> */
    public static function protectedRoutes(): iterable
    {
        self::bootKernel();
        $router = self::getContainer()->get(RouterInterface::class);

        foreach ($router->getRouteCollection() as $route) {
            $path = $route->getPath();
            if (!str_starts_with($path, '/v1')) {
                continue;
            }
            $concrete = preg_replace('#\{[^}]+\}#', 'x', $path);
            \assert(\is_string($concrete));
            $methods = [] === $route->getMethods() ? ['GET'] : $route->getMethods();
            foreach ($methods as $method) {
                if (self::isPublic($method, $concrete)) {
                    continue;
                }
                yield $method.' '.$path => [$method, $concrete];
            }
        }
        self::ensureKernelShutdown();
    }

    #[DataProvider('protectedRoutes')]
    public function testAnswers401WithoutASession(string $method, string $path): void
    {
        $client = self::createClient();
        $client->request($method, $path);

        self::assertSame(Response::HTTP_UNAUTHORIZED, $client->getResponse()->getStatusCode());
    }

    public function testTheRouterListsRoutesToCheck(): void
    {
        self::assertNotEmpty(iterator_to_array(self::protectedRoutes()));
    }

    private static function isPublic(string $method, string $path): bool
    {
        foreach (self::PUBLIC as $pattern) {
            if (1 === preg_match($pattern, $path)) {
                return true;
            }
        }

        return 'GET' === $method && 1 === preg_match(self::PUBLIC_GET, $path);
    }
}
