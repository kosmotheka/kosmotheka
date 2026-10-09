<?php

declare(strict_types=1);

namespace App\Tests\Support;

use App\Dto\ChangePayload;
use App\Dto\OperationPayload;
use App\Enum\CatalogEntity;

/**
 * A made-up library for the load tests: some members, publishers and people, then one operation per book (work,
 * edition, copy, details and an author, so the rule on repeated contributors is measured too), a title fixed on every tenth edition and a work deleted every fiftieth book.
 */
final class LibraryHistory
{
    public const int MEMBERS = 5;
    public const int PUBLISHERS = 50;
    public const int PEOPLE = 100;

    /** The HLC time of the first operation pushed on top of a history (above every operation of it). */
    public const int PUSHES_FROM = 100_000_000;

    /**
     * @return \Generator<int, OperationPayload>
     */
    public static function operations(int $books): \Generator
    {
        $clock = 0;
        $members = array_map(static fn (int $i): ChangePayload => Ops::create(CatalogEntity::Member, "m$i", ['name' => "Persona $i", 'role' => 0 === $i ? 'admin' : 'reader']), range(0, self::MEMBERS - 1));
        $publishers = array_map(static fn (int $i): ChangePayload => Ops::create(CatalogEntity::Publisher, "p$i", ['name' => "Editorial $i"]), range(0, self::PUBLISHERS - 1));
        $people = array_map(static fn (int $i): ChangePayload => Ops::create(CatalogEntity::Person, "pe$i", ['name' => "Autora $i"]), range(0, self::PEOPLE - 1));
        yield Ops::operation(Ops::hlc(++$clock), [...$members, ...$publishers, ...$people]);
        for ($i = 0; $i < $books; ++$i) {
            yield Ops::operation(Ops::hlc(++$clock), self::book("$i", 0 === $i % 7 ? null : 'p'.($i % self::PUBLISHERS), 'm'.($i % self::MEMBERS), 1950 + $i % 70, $i % self::PEOPLE));
        }
        for ($i = 0; $i < $books; $i += 10) {
            yield Ops::operation(Ops::hlc(++$clock), [Ops::update(CatalogEntity::Edition, "e$i", ['title' => "Libro $i (corregido)", 'published_year' => 2000])]);
        }
        for ($i = 5; $i < $books; $i += 50) {
            yield Ops::operation(Ops::hlc(++$clock), [Ops::delete(CatalogEntity::Work, "w$i")]);
        }
    }

    /**
     * What an ordinary push adds on top of a library: a new book, and a title fixed on an old one.
     *
     * @return list<OperationPayload>
     */
    public static function push(int $n): array
    {
        return [
            Ops::operation(Ops::hlc(self::PUSHES_FROM + 2 * $n), self::book("new-$n", 'p3', 'm1', 2024, $n % self::PEOPLE)),
            Ops::operation(Ops::hlc(self::PUSHES_FROM + 2 * $n + 1), [Ops::update(CatalogEntity::Edition, 'e'.(100 + $n), ['title' => "Retocado $n"])]),
        ];
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
            Ops::create(CatalogEntity::Work, "w$n", ['kind' => 'book', 'title' => "Libro $n"]),
            Ops::create(CatalogEntity::Edition, "e$n", ['work_id' => "w$n", 'title' => "Libro $n", 'publisher_id' => $publisher, 'published_year' => $year]),
            Ops::create(CatalogEntity::WorkContributor, "wc$n", ['work_id' => "w$n", 'person_id' => "pe$author", 'role' => 'author', 'position' => 1]),
            Ops::create(CatalogEntity::Copy, "c$n", ['edition_id' => "e$n", 'owner_id' => $owner, 'format' => 'paperback', 'quantity' => 1]),
            Ops::create(CatalogEntity::BookDetails, "e$n", ['isbn13' => \sprintf('978%010d', crc32($n))]),
        ];
    }
}
