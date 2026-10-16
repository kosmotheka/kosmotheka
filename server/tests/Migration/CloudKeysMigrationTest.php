<?php

declare(strict_types=1);

namespace App\Tests\Migration;

use App\Security\CloudKeyVault;
use App\Security\CloudKeyVaultException;
use App\Tests\Support\CatalogRows;
use Doctrine\DBAL\Connection;
use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\Exception\AbortMigration;
use Doctrine\Migrations\Exception\IrreversibleMigration;
use Doctrine\ORM\EntityManagerInterface;
use DoctrineMigrations\Version20261016120000;
use Psr\Log\NullLogger;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;

/** SEG-074: the migration that seals the cloud keys stored before the library became part of what authenticates them. */
final class CloudKeysMigrationTest extends KernelTestCase
{
    private Connection $connection;
    private string $secret;
    private ?string $env = null;

    public static function setUpBeforeClass(): void
    {
        // Migrations are loaded by Doctrine's own loader, not by the autoloader.
        require_once __DIR__.'/../../migrations/Version20261016120000.php';
    }

    protected function setUp(): void
    {
        self::bootKernel();
        $this->connection = self::getContainer()->get(Connection::class);
        $this->secret = base64_encode(random_bytes(32));
        $this->env = \is_string($_SERVER['CLOUD_KEY_SECRET'] ?? null) ? $_SERVER['CLOUD_KEY_SECRET'] : null;
        $_SERVER['CLOUD_KEY_SECRET'] = $this->secret;
    }

    protected function tearDown(): void
    {
        if (null === $this->env) {
            unset($_SERVER['CLOUD_KEY_SECRET']);
        } else {
            $_SERVER['CLOUD_KEY_SECRET'] = $this->env;
        }
        parent::tearDown();
    }

    public function testEveryOldKeyIsSealedAgainForItsLibraryAndOpensToTheSameBytes(): void
    {
        [$a, $b] = [$this->library('one'), $this->library('two')];
        $oldA = $this->legacy('private key of one');
        $oldB = $this->legacy('private key of two');
        $this->store($a, $oldA);
        $this->store($b, $oldB);

        $this->migrate();

        $vault = new CloudKeyVault($this->secret);
        foreach ([[$a, 'private key of one'], [$b, 'private key of two']] as [$id, $plain]) {
            $sealed = $this->stored($id);
            self::assertStringStartsWith('v1.', $sealed);
            self::assertSame($plain, $vault->open($id, $sealed));
        }
        // And each one is tied to its own library.
        $this->expectException(CloudKeyVaultException::class);
        $vault->open($b, $this->stored($a));
    }

    public function testWhatIsAlreadySealedAndALibraryWithoutAKeyAreNotTouched(): void
    {
        $a = $this->library('one');
        $none = $this->library('none');
        $current = new CloudKeyVault($this->secret)->seal($a, 'already new');
        $this->store($a, $current);

        $this->migrate();

        self::assertSame($current, $this->stored($a));
        self::assertNull($this->connection->fetchOne('SELECT cloud_secret_key FROM cloud_library WHERE id = ?', [$none]) ?: null);
    }

    public function testAKeyThatDoesNotOpenStopsTheMigrationAndChangesNothing(): void
    {
        [$good, $bad] = [$this->library('good'), $this->library('bad')];
        $oldGood = $this->legacy('a key that opens');
        $nonce = random_bytes(\SODIUM_CRYPTO_SECRETBOX_NONCEBYTES);
        $oldBad = sodium_bin2base64($nonce.sodium_crypto_secretbox('sealed with another secret', $nonce, random_bytes(32)), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        $this->store($good, $oldGood);
        $this->store($bad, $oldBad);

        try {
            $this->migrate();
            self::fail('The migration went on with a key that does not open.');
        } catch (AbortMigration) {
        }

        self::assertSame($oldGood, $this->stored($good), 'the good one stayed as it was too');
        self::assertSame($oldBad, $this->stored($bad));
    }

    public function testWithoutTheSecretItStopsAndChangesNothing(): void
    {
        $a = $this->library('one');
        $old = $this->legacy('a key');
        $this->store($a, $old);
        $_SERVER['CLOUD_KEY_SECRET'] = '';

        try {
            $this->migrate();
            self::fail('The migration went on without a secret.');
        } catch (AbortMigration) {
        }

        self::assertSame($old, $this->stored($a));
    }

    public function testItDoesNothingAndNeedsNoSecretWhenThereIsNothingToSeal(): void
    {
        $_SERVER['CLOUD_KEY_SECRET'] = '';
        $this->library('one');

        $this->migrate();

        $this->addToAssertionCount(1);
    }

    public function testItIsNotReversible(): void
    {
        $this->expectException(IrreversibleMigration::class);
        new Version20261016120000($this->connection, new NullLogger())->down(new Schema());
    }

    private function migrate(): void
    {
        $migration = new Version20261016120000($this->connection, new NullLogger());
        $migration->up(new Schema());
        foreach ($migration->getSql() as $query) {
            /** @var array<string, mixed> $parameters */
            $parameters = $query->getParameters();
            $this->connection->executeStatement($query->getStatement(), $parameters);
        }
    }

    private function library(string $name): string
    {
        $em = self::getContainer()->get(EntityManagerInterface::class);
        $library = new CatalogRows($em)->cloudLibrary();
        $em->flush();
        unset($name);

        return $library->getId()->toRfc4122();
    }

    private function legacy(string $plain): string
    {
        $nonce = random_bytes(\SODIUM_CRYPTO_SECRETBOX_NONCEBYTES);

        return sodium_bin2base64($nonce.sodium_crypto_secretbox($plain, $nonce, sodium_base642bin($this->secret, \SODIUM_BASE64_VARIANT_ORIGINAL)), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
    }

    private function store(string $library, string $sealed): void
    {
        $this->connection->executeStatement('UPDATE cloud_library SET cloud_secret_key = ? WHERE id = ?', [$sealed, $library]);
    }

    private function stored(string $library): string
    {
        $value = $this->connection->fetchOne('SELECT cloud_secret_key FROM cloud_library WHERE id = ?', [$library]);
        \assert(\is_string($value));

        return $value;
    }
}
