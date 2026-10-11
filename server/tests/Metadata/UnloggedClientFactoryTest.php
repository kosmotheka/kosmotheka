<?php

declare(strict_types=1);

namespace App\Tests\Metadata;

use App\Metadata\UnloggedClientFactory;
use PHPUnit\Framework\TestCase;
use Psr\Log\LoggerAwareInterface;

/**
 * RAWG and Comic Vine only take their key in the address of the call. The framework's http client writes the address of
 * every request to the log (at the `info` level), so the client they are asked with must have no logger at all.
 */
final class UnloggedClientFactoryTest extends TestCase
{
    public function testTheClientAKeyInTheAddressIsAskedWithHasNoLogger(): void
    {
        $client = UnloggedClientFactory::create();

        $loggers = [];
        for ($class = new \ReflectionObject($client); false !== $class; $class = $class->getParentClass()) {
            foreach ($class->getProperties() as $property) {
                if ('logger' === $property->getName()) {
                    $loggers[] = $property->getValue($client);
                }
            }
        }

        self::assertInstanceOf(LoggerAwareInterface::class, $client);
        self::assertNotSame([], $loggers, 'the client must keep its logger in a property called logger, or this test says nothing');
        self::assertSame([null], array_values(array_unique($loggers, \SORT_REGULAR)));
    }
}
