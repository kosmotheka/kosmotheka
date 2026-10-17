<?php

declare(strict_types=1);

namespace App\Tests\Entity;

use App\Entity\CloudOperation;
use Doctrine\DBAL\Connection;
use Doctrine\DBAL\Types\Type;
use Doctrine\ORM\EntityManagerInterface;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;

/**
 * The body is an object, not an array: an empty `{}` must not come back as `[]`, because that
 * would change what the author signed.
 */
final class CloudOperationBodyTest extends KernelTestCase
{
    public function testTheMappingSaysObject(): void
    {
        self::bootKernel();
        $em = self::getContainer()->get(EntityManagerInterface::class);

        self::assertSame('jsonb_object', $em->getClassMetadata(CloudOperation::class)->getFieldMapping('body')->type);
    }

    /**
     * @return iterable<string, array{string}>
     */
    public static function bodies(): iterable
    {
        yield 'empty object' => ['{}'];
        yield 'nested empty object' => ['{"a":{}}'];
        yield 'empty list stays a list' => ['{"a":[]}'];
    }

    #[DataProvider('bodies')]
    public function testItComesBackAsAnObjectThroughTheDatabase(string $json): void
    {
        self::bootKernel();
        $db = self::getContainer()->get(Connection::class);
        $type = Type::getType('jsonb_object');
        $platform = $db->getDatabasePlatform();

        $body = $type->convertToPHPValue($json, $platform);
        self::assertInstanceOf(\stdClass::class, $body);

        $stored = $type->convertToDatabaseValue($body, $platform);
        $fromDatabase = $db->fetchOne('SELECT CAST(CAST(:body AS JSONB) AS TEXT)', ['body' => $stored]);
        self::assertIsString($fromDatabase);

        $back = $type->convertToPHPValue($fromDatabase, $platform);
        self::assertEquals($body, $back);
        self::assertSame(json_encode($body), json_encode($back));
    }
}
