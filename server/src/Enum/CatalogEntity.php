<?php

declare(strict_types=1);

namespace App\Enum;

use App\Dto\FieldSpec;
use App\Entity\Catalog\BoardgameDetails;
use App\Entity\Catalog\BookDetails;
use App\Entity\Catalog\CatalogRow;
use App\Entity\Catalog\ChangeRequest;
use App\Entity\Catalog\ComicDetails;
use App\Entity\Catalog\Copy;
use App\Entity\Catalog\Device;
use App\Entity\Catalog\Edition;
use App\Entity\Catalog\FilmDetails;
use App\Entity\Catalog\Library;
use App\Entity\Catalog\Mark;
use App\Entity\Catalog\Member;
use App\Entity\Catalog\Person;
use App\Entity\Catalog\Publisher;
use App\Entity\Catalog\Revocation;
use App\Entity\Catalog\Series;
use App\Entity\Catalog\SeriesEntry;
use App\Entity\Catalog\Tag;
use App\Entity\Catalog\TvSeriesDetails;
use App\Entity\Catalog\Work;
use App\Entity\Catalog\WorkContributor;
use App\Entity\Catalog\WorkGenre;
use App\Entity\Catalog\WorkRelation;
use App\Entity\Catalog\WorkTag;

/**
 * Which catalog entities operations may change, and the fields each one
 * accepts. Mirrors spec/operation.schema.json and crates/domain/src/schema.rs;
 * keep the three in sync (the conformance cases catch most drifts).
 *
 * "Catalog entity", not "entity": it names what an operation changes in the
 * app's vocabulary; the Doctrine class of each one is entityClass() (D92).
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
        'novel', 'science_fiction', 'fantasy', 'horror', 'mystery', 'historical',
        'romance', 'adventure', 'poetry', 'theatre', 'essay', 'biography',
        'popular_science', 'children', 'young_adult', 'roleplaying', 'cooking',
        'art', 'travel', 'others', 'superheroes', 'manga', 'humor', 'graphic_novel',
        'crime', 'strategy', 'family', 'party', 'cooperative', 'abstract',
        'card_game', 'tile_placement', 'worker_placement', 'deck_building',
        'deduction', 'wargame', 'action', 'animation', 'comedy', 'documentary',
        'drama', 'musical', 'thriller', 'war', 'western', 'platformer', 'puzzle',
        'racing', 'sports', 'fighting', 'shooter', 'simulation', 'rock', 'pop',
        'jazz', 'classical', 'electronic', 'hip_hop', 'folk', 'flamenco', 'blues',
        'soul', 'metal', 'soundtrack',
    ];

    /** The PostgreSQL table of the entity (milestone 7.4.3); `catalog_` keeps them apart from the server's own. */
    public function table(): string
    {
        return 'catalog_'.match ($this) {
            self::Library => 'libraries',
            self::Work => 'works',
            self::Person => 'people',
            self::WorkContributor => 'work_contributors',
            self::Edition => 'editions',
            self::BookDetails => 'book_details',
            self::ComicDetails => 'comic_details',
            self::BoardgameDetails => 'boardgame_details',
            self::FilmDetails => 'film_details',
            self::TvSeriesDetails => 'tv_series_details',
            self::Member => 'members',
            self::Copy => 'copies',
            self::Publisher => 'publishers',
            self::Tag => 'tags',
            self::WorkTag => 'work_tags',
            self::WorkGenre => 'work_genres',
            self::Series => 'series',
            self::SeriesEntry => 'series_entries',
            self::WorkRelation => 'work_relations',
            self::Revocation => 'revocations',
            self::Device => 'devices',
            self::ChangeRequest => 'change_requests',
            self::Mark => 'marks',
        };
    }

    /** Whether the row is the details of an edition (same id as its edition). */
    public function isDetails(): bool
    {
        return \in_array($this, [self::BookDetails, self::ComicDetails, self::BoardgameDetails, self::FilmDetails, self::TvSeriesDetails], true);
    }

    /**
     * The Doctrine entity of the row (D92).
     *
     * @return class-string<CatalogRow>
     */
    public function entityClass(): string
    {
        return match ($this) {
            self::Library => Library::class,
            self::Work => Work::class,
            self::Person => Person::class,
            self::WorkContributor => WorkContributor::class,
            self::Edition => Edition::class,
            self::BookDetails => BookDetails::class,
            self::ComicDetails => ComicDetails::class,
            self::BoardgameDetails => BoardgameDetails::class,
            self::FilmDetails => FilmDetails::class,
            self::TvSeriesDetails => TvSeriesDetails::class,
            self::Member => Member::class,
            self::Copy => Copy::class,
            self::Publisher => Publisher::class,
            self::Tag => Tag::class,
            self::WorkTag => WorkTag::class,
            self::WorkGenre => WorkGenre::class,
            self::Series => Series::class,
            self::SeriesEntry => SeriesEntry::class,
            self::WorkRelation => WorkRelation::class,
            self::Revocation => Revocation::class,
            self::Device => Device::class,
            self::ChangeRequest => ChangeRequest::class,
            self::Mark => Mark::class,
        };
    }

    /**
     * @return array<string, FieldSpec>
     */
    public function fields(): array
    {
        $text = static fn (bool $required = false): FieldSpec => new FieldSpec(FieldType::Text, $required);
        $ref = static fn (self $target, bool $required = false): FieldSpec => new FieldSpec(FieldType::Ref, $required, [], $target);
        $integer = static fn (bool $required = false): FieldSpec => new FieldSpec(FieldType::Integer, $required);
        $oneOf = static fn (string ...$allowed): FieldSpec => new FieldSpec(FieldType::OneOf, true, array_values($allowed));

        return match ($this) {
            self::Work => ['kind' => $oneOf(...self::KINDS), 'title' => $text(true)],
            self::Library => ['name' => $text(true), 'cloud_library_id' => $text(), 'cloud_server' => $text(), 'cloud_device_id' => new FieldSpec(FieldType::DeviceIdOrEmpty, false), 'founding_nonce' => $text()],
            self::Person, self::Publisher, self::Tag => ['name' => $text(true)],
            self::Member => [
                'name' => $text(true),
                'role' => new FieldSpec(FieldType::OneOf, false, self::ROLES),
                'can_share' => new FieldSpec(FieldType::Boolean, false),
                'account_id' => $text(),
                'access_withdrawn' => new FieldSpec(FieldType::Boolean, false),
            ],
            self::WorkContributor => [
                'work_id' => $ref(self::Work, true),
                'person_id' => $ref(self::Person, true),
                'role' => $oneOf(...self::CONTRIBUTOR_ROLES),
                'position' => $integer(true),
            ],
            self::Edition => [
                'work_id' => $ref(self::Work, true),
                'title' => $text(true),
                'subtitle' => $text(),
                'publisher_id' => $ref(self::Publisher),
                'published_year' => $integer(),
                'language' => $text(),
                'cover' => $text(),
                'barcode' => $text(),
                'notes' => $text(),
            ],
            self::BookDetails => [
                'isbn13' => new FieldSpec(FieldType::Isbn13, false),
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
                'edition_id' => $ref(self::Edition, true),
                'owner_id' => $ref(self::Member),
                'format' => $oneOf(...self::FORMATS),
                'quantity' => new FieldSpec(FieldType::Count, true),
                'location' => $text(),
                'acquired_on' => new FieldSpec(FieldType::Date, false),
                // A video game copy's platform: belongs to the copy, not the edition.
                'platform' => $text(),
                // A TV series copy's held seasons, as free text ("1-2", "1-3, 5").
                'seasons' => $text(),
            ],
            self::WorkTag => [
                'work_id' => $ref(self::Work, true),
                'tag_id' => $ref(self::Tag, true),
            ],
            self::WorkGenre => [
                'work_id' => $ref(self::Work, true),
                'genre' => $oneOf(...self::GENRES),
            ],
            self::Series => [
                'name' => $text(true),
                'ordered' => new FieldSpec(FieldType::Boolean, true),
            ],
            self::SeriesEntry => [
                'series_id' => $ref(self::Series, true),
                'work_id' => $ref(self::Work, true),
                'position' => $integer(),
            ],
            self::WorkRelation => [
                'work_id' => $ref(self::Work, true),
                'related_work_id' => $ref(self::Work, true),
                'relation' => $oneOf(...self::WORK_RELATIONS),
            ],
            self::Revocation => ['device_id' => new FieldSpec(FieldType::DeviceId, true), 'device_name' => $text(true)],
            self::Device => ['public_key' => $text(true), 'name' => $text(true), 'member_id' => $ref(self::Member), 'trusted_through' => $text(), 'vouched_key' => $text()],
            self::ChangeRequest => [
                'status' => new FieldSpec(FieldType::OneOf, false, ['accepted', 'rejected', 'withdrawn']),
                'note' => $text(),
            ],
            self::Mark => [
                'member_id' => $ref(self::Member, true),
                'work_id' => $ref(self::Work, true),
                'status' => new FieldSpec(FieldType::OneOf, false, self::MARK_STATUSES),
                'status_on' => new FieldSpec(FieldType::Date, false),
                'rating' => new FieldSpec(FieldType::Rating, false),
                'favorite' => new FieldSpec(FieldType::Boolean, false),
                'wished' => new FieldSpec(FieldType::Boolean, false),
            ],
        };
    }
}
