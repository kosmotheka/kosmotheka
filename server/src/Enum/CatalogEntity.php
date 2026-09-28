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
    case Library = 'library';
    case Work = 'work';
    case Person = 'person';
    case WorkContributor = 'work_contributor';
    case Edition = 'edition';
    case BookDetails = 'book_details';
    case ComicDetails = 'comic_details';
    case BoardgameDetails = 'boardgame_details';
    case FilmDetails = 'film_details';
    case TvSeriesDetails = 'tv_series_details';
    case VideogameDetails = 'videogame_details';
    case Member = 'member';
    case Copy = 'copy';
    case Publisher = 'publisher';
    case Wish = 'wish';
    case Revocation = 'revocation';
    case Device = 'device';
    case ChangeRequest = 'change_request';
    case Mark = 'mark';

    public const array FORMATS = [
        'hardcover',
        'paperback',
        'pocket',
        'ebook',
        'pdf',
        'audiobook',
        'single_issue',
        'box',
        'dvd',
        'bluray',
        'uhd',
        'physical',
        'digital',
        'vinyl',
        'cd',
        'cassette',
        'other',
    ];

    /** What a work is (D60). */
    public const array KINDS = ['book', 'comic', 'boardgame', 'film', 'tv_series', 'videogame', 'music', 'other'];

    /** What someone did for a work. */
    public const array CONTRIBUTOR_ROLES = ['author', 'writer', 'illustrator', 'designer', 'director', 'artist'];

    /** Roles a member can have in a library (D55, D73). */
    public const array ROLES = ['admin', 'editor', 'reader'];

    /** Where a person is with a work (D68). */
    public const array MARK_STATUSES = ['pending', 'in_progress', 'finished', 'abandoned'];

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
            self::Work => ['kind' => $oneOf(...self::KINDS), 'title' => $text(true)],
            self::Library, self::Person, self::Publisher => ['name' => $text(true)],
            self::Member => [
                'name' => $text(true),
                'role' => new FieldSpec(FieldType::OneOf, false, self::ROLES),
                'can_share' => new FieldSpec(FieldType::Boolean, false),
            ],
            self::WorkContributor => [
                'work_id' => $ref(true),
                'person_id' => $ref(true),
                'role' => $oneOf(...self::CONTRIBUTOR_ROLES),
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
                'barcode' => $text(),
            ],
            self::BookDetails => [
                'isbn13' => $text(),
                'google_books_id' => $text(),
                'open_library_id' => $text(),
            ],
            self::ComicDetails => ['number' => $text()],
            self::BoardgameDetails => [
                'players_min' => $integer(),
                'players_max' => $integer(),
                'duration_minutes' => $integer(),
                'min_age' => $integer(),
            ],
            self::FilmDetails => ['duration_minutes' => $integer()],
            self::TvSeriesDetails => ['seasons' => $integer()],
            self::VideogameDetails => ['platform' => $text()],
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
            self::Revocation => ['device_id' => $text(true), 'device_name' => $text(true)],
            self::Device => ['public_key' => $text(true), 'name' => $text(true), 'member_id' => $ref()],
            self::ChangeRequest => [
                'status' => new FieldSpec(FieldType::OneOf, false, ['accepted', 'rejected', 'withdrawn']),
                'note' => $text(),
            ],
            self::Mark => [
                'member_id' => $ref(true),
                'work_id' => $ref(true),
                'status' => new FieldSpec(FieldType::OneOf, false, self::MARK_STATUSES),
                'status_on' => new FieldSpec(FieldType::Date, false),
                'rating' => new FieldSpec(FieldType::Rating, false),
                'favorite' => new FieldSpec(FieldType::Boolean, false),
            ],
        };
    }
}
