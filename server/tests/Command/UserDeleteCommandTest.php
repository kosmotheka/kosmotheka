<?php

declare(strict_types=1);

namespace App\Tests\Command;

use App\Entity\Catalog\Member;
use App\Entity\CloudLibrary;
use App\Entity\Membership;
use App\Entity\User;
use App\Sync\CloudLibraryCreator;
use Doctrine\DBAL\Connection;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Console\Application;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Tester\CommandTester;
use Symfony\Component\Uid\Uuid;

/**
 * Deleting an account from the console does what the route does (D99, D66): the libraries it uploaded pass to someone
 * else with a place in them, or go with it, and the log keeps one line without the email.
 */
final class UserDeleteCommandTest extends KernelTestCase
{
    protected function setUp(): void
    {
        self::bootKernel();
    }

    public function testALibraryWithAnotherPlaceStaysAndPassesToThatAccount(): void
    {
        $ana = $this->user('ana');
        $bea = $this->user('bea');
        $library = $this->library($ana);
        $this->place($library, $bea, 'admin');

        $tester = $this->delete($ana->getEmail(), ['--force' => true]);

        $tester->assertCommandIsSuccessful();
        self::assertSame(0, $this->rows('"user"', 'id', $ana->getId()->toRfc4122()), 'The account is gone.');
        self::assertSame(1, $this->rows('cloud_library', 'id', $library->getId()->toRfc4122()), 'The library stays.');
        self::assertSame($bea->getId()->toRfc4122(), $this->connection()->fetchOne('SELECT owner_id FROM cloud_library WHERE id = ?', [$library->getId()->toRfc4122()]));
    }

    public function testALibraryNobodyElseCanReachGoesWithTheAccount(): void
    {
        $ana = $this->user('ana');
        $library = $this->library($ana);

        $this->delete($ana->getEmail(), ['--force' => true])->assertCommandIsSuccessful();

        self::assertSame(0, $this->rows('"user"', 'id', $ana->getId()->toRfc4122()));
        self::assertSame(0, $this->rows('cloud_library', 'id', $library->getId()->toRfc4122()));
        self::assertSame(0, $this->rows('membership', 'cloud_library_id', $library->getId()->toRfc4122()));
    }

    public function testAnUnknownEmailFailsAndTouchesNothing(): void
    {
        $ana = $this->user('ana');
        $before = $this->rows('activity_log', 'kind', 'account_deleted');

        $tester = $this->delete('nobody-'.bin2hex(random_bytes(4)).'@example.com', ['--force' => true]);

        self::assertSame(Command::FAILURE, $tester->getStatusCode());
        self::assertSame(1, $this->rows('"user"', 'id', $ana->getId()->toRfc4122()));
        self::assertSame($before, $this->rows('activity_log', 'kind', 'account_deleted'));
    }

    public function testTheLogKeepsTheLineWithTheIdAndNeverTheEmail(): void
    {
        $ana = $this->user('ana');
        $id = $ana->getId()->toRfc4122();
        $email = $ana->getEmail();

        $this->delete($email, ['--force' => true])->assertCommandIsSuccessful();

        $rows = $this->connection()->fetchAllAssociative("SELECT * FROM activity_log WHERE kind = 'account_deleted' AND account_id = ?", [$id]);
        self::assertCount(1, $rows);
        self::assertSame('console', $rows[0]['channel']);
        self::assertNotNull($rows[0]['account_email_fingerprint']);
        self::assertStringNotContainsString($email, json_encode($rows, \JSON_THROW_ON_ERROR));
    }

    public function testWithoutForceItShowsWhatWillHappenAndAsksFirst(): void
    {
        $ana = $this->user('ana');
        $bea = $this->user('bea');
        $shared = $this->library($ana);
        $this->place($shared, $bea, 'admin');
        $alone = $this->library($ana);

        $tester = $this->delete($ana->getEmail(), [], ['no']);

        $output = $tester->getDisplay();
        self::assertStringContainsString($ana->getEmail(), $output);
        self::assertStringContainsString($shared->getId()->toRfc4122(), $output);
        self::assertStringContainsString($alone->getId()->toRfc4122(), $output);
        self::assertSame(1, $this->rows('"user"', 'id', $ana->getId()->toRfc4122()), 'Answering no deletes nothing.');
        self::assertSame(2, $this->rows('cloud_library', 'owner_id', $ana->getId()->toRfc4122()));
    }

    public function testAnsweringYesDeletes(): void
    {
        $ana = $this->user('ana');

        $this->delete($ana->getEmail(), [], ['yes'])->assertCommandIsSuccessful();

        self::assertSame(0, $this->rows('"user"', 'id', $ana->getId()->toRfc4122()));
    }

    /**
     * @param array<string, mixed> $options
     * @param list<string>         $inputs
     */
    private function delete(string $email, array $options, array $inputs = []): CommandTester
    {
        $tester = new CommandTester(new Application(self::$kernel ?? self::bootKernel())->find('app:user:delete'));
        if ([] !== $inputs) {
            $tester->setInputs($inputs);
        }
        $tester->execute(['email' => $email] + $options, [] === $inputs ? ['interactive' => false] : []);

        return $tester;
    }

    private function em(): EntityManagerInterface
    {
        return self::getContainer()->get(EntityManagerInterface::class);
    }

    private function connection(): Connection
    {
        return self::getContainer()->get(Connection::class);
    }

    private function user(string $name): User
    {
        $user = new User(\sprintf('%s-%s@example.com', $name, bin2hex(random_bytes(4))), ucfirst($name), 'en');
        $this->em()->persist($user);
        $this->em()->flush();

        return $user;
    }

    private function library(User $owner): CloudLibrary
    {
        $library = self::getContainer()->get(CloudLibraryCreator::class)->create($owner);
        $this->em()->flush();

        return $library;
    }

    private function place(CloudLibrary $library, User $user, string $role): void
    {
        $person = Uuid::v7();
        $member = new Member($library, $person->toRfc4122());
        $member->setName($user->getName());
        $member->setRole($role);
        $this->em()->persist($member);
        $this->em()->persist(new Membership($user, $library, $person));
        $this->em()->flush();
    }

    private function rows(string $table, string $column, string $value): int
    {
        $count = $this->connection()->fetchOne("SELECT COUNT(*) FROM $table WHERE $column = ?", [$value]);
        \assert(\is_int($count) || \is_string($count));

        return (int) $count;
    }
}
