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
    case Member = 'member';
    case Copy = 'copy';
    case Publisher = 'publisher';
    case Tag = 'tag';
    case WorkTag = 'work_tag';
    case WorkGenre = 'work_genre';
    case Wish = 'wish';
    case Series = 'series';
    case SeriesEntry = 'series_entry';
    case WorkRelation = 'work_relation';
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
    public const array CONTRIBUTOR_ROLES = ['author', 'writer', 'illustrator', 'designer', 'director', 'artist', 'studio'];

    /** Roles a member can have in a library (D55, D73). */
    public const array ROLES = ['admin', 'editor', 'reader'];

    /** Where a person is with a work (D68). */
    public const array MARK_STATUSES = ['pending', 'in_progress', 'finished', 'abandoned'];

    /** What one work may say about another (D61). */
    public const array WORK_RELATIONS = ['requires'];

    /** The closed list of genres shipped with the app, per kind (mirrors crates/domain/src/genre.rs). */
    public const array GENRES = [
        'fantasy', 'science_fiction', 'horror', 'mystery', 'thriller', 'romance',
        'historical_fiction', 'adventure', 'humor', 'poetry', 'theatre', 'essay',
        'biography', 'history', 'science', 'philosophy', 'art', 'cooking', 'travel',
        'roleplaying', 'children', 'young_adult', 'superheroes', 'manga', 'crime',
        'slice_of_life', 'strategy', 'family', 'party', 'cooperative', 'abstract',
        'deduction', 'wargame', 'card_game', 'dexterity', 'trivia', 'action',
        'animation', 'comedy', 'drama', 'war', 'western', 'musical', 'documentary',
        'simulation', 'sports', 'racing', 'puzzle', 'platformer', 'shooter',
        'fighting', 'rock', 'pop', 'jazz', 'classical', 'electronic', 'hip_hop',
        'folk', 'metal', 'blues', 'soul', 'reggae', 'latin', 'flamenco',
        'soundtrack', 'world',
    ];

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
            self::Library, self::Person, self::Publisher, self::Tag => ['name' => $text(true)],
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
                'notes' => $text(),
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
            self::Copy => [
                'edition_id' => $ref(true),
                'owner_id' => $ref(),
                'format' => $oneOf(...self::FORMATS),
                'quantity' => $integer(true),
                'location' => $text(),
                'acquired_on' => new FieldSpec(FieldType::Date, false),
                // A video game copy's platform: belongs to the copy, not the edition.
                'platform' => $text(),
                // A TV series copy's held seasons, as free text ("1-2", "1-3, 5").
                'seasons' => $text(),
            ],
            self::WorkTag => [
                'work_id' => $ref(true),
                'tag_id' => $ref(true),
            ],
            self::WorkGenre => [
                'work_id' => $ref(true),
                'genre' => $oneOf(...self::GENRES),
            ],
            self::Wish => [
                'title' => $text(true),
                'author' => $text(),
                'isbn13' => $text(),
                'note' => $text(),
                'cover_url' => $text(),
            ],
            self::Series => [
                'name' => $text(true),
                'ordered' => new FieldSpec(FieldType::Boolean, true),
            ],
            self::SeriesEntry => [
                'series_id' => $ref(true),
                'work_id' => $ref(true),
                'position' => $integer(),
            ],
            self::WorkRelation => [
                'work_id' => $ref(true),
                'related_work_id' => $ref(true),
                'relation' => $oneOf(...self::WORK_RELATIONS),
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
                'wished' => new FieldSpec(FieldType::Boolean, false),
            ],
        };
    }
}
