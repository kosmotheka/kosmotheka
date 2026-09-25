<?php

declare(strict_types=1);

namespace App\Enum;

use App\Dto\FieldSpec;

/**
 * Which catalog entities operations may change, and the fields each one
 * accepts. Mirrors spec/operation.schema.json and crates/domain/src/schema.rs;
 * keep the three in sync (the conformance cases catch most drifts).
 *
 * "Catalog entity", not "entity": these are rows the operations touch, not
 * Doctrine entities.
 */
enum CatalogEntity: string
{
    case Work = 'work';
    case Person = 'person';
    case WorkContributor = 'work_contributor';
    case Edition = 'edition';
    case BookDetails = 'book_details';
    case Member = 'member';
    case Copy = 'copy';
    case Publisher = 'publisher';
    case Wish = 'wish';

    public const array FORMATS = ['hardcover', 'paperback', 'pocket', 'ebook', 'audiobook', 'other'];

    /**
     * @return array<string, FieldSpec>
     */
    public function fields(): array
    {
        $text = static fn (bool $required = false): FieldSpec => new FieldSpec(FieldType::Text, $required);
        $ref = static fn (bool $required = false): FieldSpec => new FieldSpec(FieldType::Ref, $required);
        $integer = static fn (bool $required = false): FieldSpec => new FieldSpec(FieldType::Integer, $required);
        $oneOf = static fn (string ...$allowed): FieldSpec => new FieldSpec(FieldType::OneOf, true, array_values($allowed));

        return match ($this) {
            self::Work => ['kind' => $oneOf('book'), 'title' => $text(true)],
            self::Person, self::Member, self::Publisher => ['name' => $text(true)],
            self::WorkContributor => [
                'work_id' => $ref(true),
                'person_id' => $ref(true),
                'role' => $oneOf('author'),
                'position' => $integer(true),
            ],
            self::Edition => [
                'work_id' => $ref(true),
                'title' => $text(true),
                'subtitle' => $text(),
                'publisher_id' => $ref(),
                'published_year' => $integer(),
                'language' => $text(),
                'cover' => $text(),
            ],
            self::BookDetails => [
                'isbn13' => $text(),
                'google_books_id' => $text(),
                'open_library_id' => $text(),
            ],
            self::Copy => [
                'edition_id' => $ref(true),
                'owner_id' => $ref(),
                'format' => $oneOf(...self::FORMATS),
                'quantity' => $integer(true),
                'location' => $text(),
                'acquired_on' => new FieldSpec(FieldType::Date, false),
            ],
            self::Wish => [
                'title' => $text(true),
                'author' => $text(),
                'isbn13' => $text(),
                'note' => $text(),
                'cover_url' => $text(),
            ],
        };
    }
}
