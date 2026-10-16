<?php

declare(strict_types=1);

namespace App\Tests\Support;

use App\DataFixtures\SigningDevice;
use App\Dto\ChangePayload;
use App\Dto\OperationPayload;
use App\Enum\CatalogEntity;
use Symfony\Component\Uid\Uuid;

/**
 * A made-up library for the load tests: some members, publishers and people, then one operation per book (work,
 * edition, copy, details and an author, so the rule on repeated contributors is measured too), a title fixed on every tenth edition and a work deleted every fiftieth book.
 */
final class LibraryHistory
{
    /** With roles, the administrator writes the even books and the editor the odd ones (and what they delete). */
    public const string ADMIN_DEVICE = '01997c7e-0000-7000-8000-0000000000fa';
    public const string EDITOR_DEVICE = '01997c7e-0000-7000-8000-0000000000fb';

    /**
     * The key of the administrator's device, from the seed every load test signs with (32 bytes of 0x01), and the library
     * it founds with SigningDevice::FOUNDING_NONCE (D97): the vector of rule 1 in spec/README.md.
     */
    public const string ADMIN_KEY = 'iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w';
    public const string LIBRARY = '2c33de53-4834-8a05-b014-c42dc60bd4e1';

    public const int MEMBERS = 5;
    public const int PUBLISHERS = 50;
    public const int PEOPLE = 100;

    /** The HLC time of the first operation pushed on top of a history (above every operation of it). */
    public const int PUSHES_FROM = 100_000_000;

    /**
     * @param bool $roles whether an editor, on a device of its own, writes half of it; without, the administrator who
     *                    founded the library writes it all
     *
     * @return \Generator<int, OperationPayload>
     */
    public static function operations(int $books, bool $roles = false): \Generator
    {
        $clock = 0;
        $roleOf = static fn (int $i): array => ['role' => match ($i) {
            0 => 'admin', 1 => 'editor', default => 'reader',
        }, 'can_share' => 0 === $i];
        yield self::operation(++$clock, self::ADMIN_DEVICE, [
            Ops::create(CatalogEntity::Library, self::LIBRARY, ['name' => 'Carga', 'founding_nonce' => SigningDevice::FOUNDING_NONCE]),
            Ops::create(CatalogEntity::Member, self::id('m', 0), ['name' => 'Persona 0'] + $roleOf(0)),
            Ops::create(CatalogEntity::Device, self::ADMIN_DEVICE, ['public_key' => self::ADMIN_KEY, 'name' => 'Admin', 'member_id' => self::id('m', 0), 'vouched_key' => self::ADMIN_KEY]),
        ]);
        $members = array_map(static fn (int $i): ChangePayload => Ops::create(CatalogEntity::Member, self::id('m', $i), ['name' => "Persona $i"] + $roleOf($i)), range(1, self::MEMBERS - 1));
        // The editor's device registers itself (only a device creates its own row); the administrator then says whose it is.
        if ($roles) {
            yield self::operation(++$clock, self::EDITOR_DEVICE, [Ops::create(CatalogEntity::Device, self::EDITOR_DEVICE, ['public_key' => self::ADMIN_KEY, 'name' => 'Editor'])]);
        }
        $devices = $roles ? [
            Ops::update(CatalogEntity::Device, self::EDITOR_DEVICE, ['member_id' => self::id('m', 1), 'vouched_key' => self::ADMIN_KEY]),
        ] : [];
        $publishers = array_map(static fn (int $i): ChangePayload => Ops::create(CatalogEntity::Publisher, self::id('p', $i), ['name' => "Editorial $i"]), range(0, self::PUBLISHERS - 1));
        $people = array_map(static fn (int $i): ChangePayload => Ops::create(CatalogEntity::Person, self::id('pe', $i), ['name' => "Autora $i"]), range(0, self::PEOPLE - 1));
        yield self::operation(++$clock, self::ADMIN_DEVICE, [...$members, ...$devices, ...$publishers, ...$people]);
        for ($i = 0; $i < $books; ++$i) {
            yield self::operation(++$clock, self::author($i, $roles), self::book("$i", 0 === $i % 7 ? null : self::id('p', $i % self::PUBLISHERS), self::id('m', $i % self::MEMBERS), 1950 + $i % 70, $i % self::PEOPLE));
        }
        for ($i = 0; $i < $books; $i += 10) {
            yield self::operation(++$clock, self::author($i, $roles), [Ops::update(CatalogEntity::Edition, self::id('e', $i), ['title' => "Libro $i (corregido)", 'published_year' => 2000])]);
        }
        for ($i = 5; $i < $books; $i += 50) {
            yield self::operation(++$clock, self::author($i, $roles), [Ops::delete(CatalogEntity::Work, self::id('w', $i))]);
        }
    }

    /**
     * What an ordinary push adds on top of a library: a new book, and a title fixed on an old one.
     *
     * @return list<OperationPayload>
     */
    public static function push(int $n, bool $roles = false): array
    {
        return [
            self::operation(self::PUSHES_FROM + 2 * $n, self::author($n, $roles), self::book("new-$n", self::id('p', 3), self::id('m', 1), 2024, $n % self::PEOPLE)),
            self::operation(self::PUSHES_FROM + 2 * $n + 1, self::author($n, $roles), [Ops::update(CatalogEntity::Edition, self::id('e', 100 + $n), ['title' => "Retocado $n"])]),
        ];
    }

    /**
     * The id of a row of the history: a UUID, as the app's are, that says its kind and number.
     *
     * @param int|string $n the number of the row, or `new-N` for the books the pushes add
     */
    public static function id(string $kind, int|string $n): string
    {
        $number = \is_string($n) && str_starts_with($n, 'new-') ? 0x1_0000_0000 + (int) substr($n, 4) : (int) $n;

        return \sprintf('01997c7e-%04x-7000-8000-%012x', array_search($kind, ['m', 'p', 'pe', 'w', 'e', 'wc', 'c'], true) + 1, $number);
    }

    private static function author(int $book, bool $roles): string
    {
        return $roles && 1 === $book % 2 ? self::EDITOR_DEVICE : self::ADMIN_DEVICE;
    }

    /**
     * @param list<ChangePayload> $changes
     */
    private static function operation(int $time, string $device, array $changes): OperationPayload
    {
        return new OperationPayload(Uuid::v7()->toRfc4122(), self::LIBRARY, Ops::hlc($time, 0, $device), 'test.op', $changes, null, Judged::SIGNATURE);
    }

    /**
     * The works (and with them editions, copies and details) still there after the history, and those deleted.
     *
     * @return array{kept: int, deleted: int}
     */
    public static function expectedBooks(int $books): array
    {
        $deleted = $books > 5 ? intdiv($books - 5 - 1, 50) + 1 : 0;

        return ['kept' => $books - $deleted, 'deleted' => $deleted];
    }

    /**
     * @return list<ChangePayload>
     */
    private static function book(string $n, ?string $publisher, string $owner, int $year, int $author): array
    {
        return [
            Ops::create(CatalogEntity::Work, self::id('w', $n), ['kind' => 'book', 'title' => "Libro $n"]),
            Ops::create(CatalogEntity::Edition, self::id('e', $n), ['work_id' => self::id('w', $n), 'title' => "Libro $n", 'publisher_id' => $publisher, 'published_year' => $year]),
            Ops::create(CatalogEntity::WorkContributor, self::id('wc', $n), ['work_id' => self::id('w', $n), 'person_id' => self::id('pe', $author), 'role' => 'author', 'position' => 1]),
            Ops::create(CatalogEntity::Copy, self::id('c', $n), ['edition_id' => self::id('e', $n), 'owner_id' => $owner, 'format' => 'paperback', 'quantity' => 1]),
            Ops::create(CatalogEntity::BookDetails, self::id('e', $n), ['isbn13' => \sprintf('978%010d', crc32($n))]),
        ];
    }
}
