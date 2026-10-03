//! Which entities operations may change, and the fields each one accepts.
//! Mirrors spec/operation.schema.json; keep both in sync.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Entity {
    /// The unit that is shared, synced and permissioned (D54). Its id is the library id.
    Library,
    Work,
    Person,
    WorkContributor,
    Edition,
    BookDetails,
    /// The other kinds' own data, one table each like `BookDetails` (D08).
    ComicDetails,
    BoardgameDetails,
    FilmDetails,
    TvSeriesDetails,
    Member,
    Copy,
    Publisher,
    /// A free word a library creates for itself, linked to works.
    Tag,
    /// A tag linked to a work; no unique constraint (D25: two devices may add the
    /// same one offline, and both rows must survive).
    WorkTag,
    /// A genre linked to a work, from the closed list in `genre::GENRES`; same
    /// no-unique-constraint rule as `WorkTag`.
    WorkGenre,
    /// A group of works (D61): a saga (ordered) or a product line (not).
    Series,
    /// A work linked to a series; no unique constraint (same rule as `WorkTag`).
    SeriesEntry,
    /// One work requiring another (an expansion requires its base game); no
    /// unique constraint (same rule as `WorkTag`).
    WorkRelation,
    /// A device whose access to the library was withdrawn (D71).
    Revocation,
    /// A device that writes to the library, with the public key its operations
    /// are signed with (D56). Its id is the device id.
    Device,
    /// How an administrator resolved a request (D73). Its id is the id of the
    /// operation that was requested.
    ChangeRequest,
    /// What one person notes about one work (D68). Its id is derived from both
    /// (`mark::mark_id`).
    Mark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Text,
    Integer,
    /// `YYYY-MM-DD`.
    Date,
    OneOf(&'static [&'static str]),
    /// `true` or `false`.
    Boolean,
    /// A whole number from 1 to 5.
    Rating,
    /// Id of another entity; the database enforces that it exists.
    Ref(Entity),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldSpec {
    pub name: &'static str,
    pub field_type: FieldType,
    pub required: bool,
}

const fn field(name: &'static str, field_type: FieldType, required: bool) -> FieldSpec {
    FieldSpec {
        name,
        field_type,
        required,
    }
}

const FORMATS: &[&str] = &[
    "hardcover",
    "paperback",
    "pocket",
    "ebook",
    "pdf",
    "audiobook",
    "single_issue",
    "box",
    "dvd",
    "bluray",
    "uhd",
    "physical",
    "digital",
    "vinyl",
    "cd",
    "cassette",
    "other",
];

/// What a work is (D60). Each kind has its own details and formats in the UI.
pub const KINDS: &[&str] = &[
    "book",
    "comic",
    "boardgame",
    "film",
    "tv_series",
    "videogame",
    "music",
    "other",
];
/// What someone did for a work: writing, drawing, designing, directing, performing.
pub const CONTRIBUTOR_ROLES: &[&str] = &[
    "author",
    "writer",
    "illustrator",
    "designer",
    "director",
    "artist",
    "studio",
];

// Declared as constants so the slices live for the whole program ('static).
const WORK_FIELDS: &[FieldSpec] = &[
    field("kind", FieldType::OneOf(KINDS), true),
    field("title", FieldType::Text, true),
];
const LIBRARY_FIELDS: &[FieldSpec] = &[field("name", FieldType::Text, true)];
const PERSON_FIELDS: &[FieldSpec] = &[field("name", FieldType::Text, true)];
const WORK_CONTRIBUTOR_FIELDS: &[FieldSpec] = &[
    field("work_id", FieldType::Ref(Entity::Work), true),
    field("person_id", FieldType::Ref(Entity::Person), true),
    field("role", FieldType::OneOf(CONTRIBUTOR_ROLES), true),
    field("position", FieldType::Integer, true),
];
const EDITION_FIELDS: &[FieldSpec] = &[
    field("work_id", FieldType::Ref(Entity::Work), true),
    field("title", FieldType::Text, true),
    field("subtitle", FieldType::Text, false),
    field("publisher_id", FieldType::Ref(Entity::Publisher), false),
    field("published_year", FieldType::Integer, false),
    field("language", FieldType::Text, false),
    field("cover", FieldType::Text, false),
    field("barcode", FieldType::Text, false),
    field("notes", FieldType::Text, false),
];
const BOOK_DETAILS_FIELDS: &[FieldSpec] = &[
    field("isbn13", FieldType::Text, false),
    field("google_books_id", FieldType::Text, false),
    field("open_library_id", FieldType::Text, false),
];
const COMIC_DETAILS_FIELDS: &[FieldSpec] = &[field("number", FieldType::Text, false)];
const BOARDGAME_DETAILS_FIELDS: &[FieldSpec] = &[
    field("players_min", FieldType::Integer, false),
    field("players_max", FieldType::Integer, false),
    field("duration_minutes", FieldType::Integer, false),
    field("min_age", FieldType::Integer, false),
];
const FILM_DETAILS_FIELDS: &[FieldSpec] = &[field("duration_minutes", FieldType::Integer, false)];
const TV_SERIES_DETAILS_FIELDS: &[FieldSpec] = &[field("seasons", FieldType::Integer, false)];
/// The entities that hang from an edition by its id, one per kind that has data of its own.
pub const DETAILS: [Entity; 5] = [
    Entity::BookDetails,
    Entity::ComicDetails,
    Entity::BoardgameDetails,
    Entity::FilmDetails,
    Entity::TvSeriesDetails,
];
/// Roles a member can have in a library (D55, D73).
pub const ROLES: &[&str] = &["admin", "editor", "reader"];
const MEMBER_FIELDS: &[FieldSpec] = &[
    field("name", FieldType::Text, true),
    field("role", FieldType::OneOf(ROLES), false),
    field("can_share", FieldType::Boolean, false),
];
const PUBLISHER_FIELDS: &[FieldSpec] = &[field("name", FieldType::Text, true)];
const TAG_FIELDS: &[FieldSpec] = &[field("name", FieldType::Text, true)];
const WORK_TAG_FIELDS: &[FieldSpec] = &[
    field("work_id", FieldType::Ref(Entity::Work), true),
    field("tag_id", FieldType::Ref(Entity::Tag), true),
];
const WORK_GENRE_FIELDS: &[FieldSpec] = &[
    field("work_id", FieldType::Ref(Entity::Work), true),
    field("genre", FieldType::OneOf(crate::genre::GENRES), true),
];
/// What one work may say about another (D61): only "requires" for now.
pub const WORK_RELATIONS: &[&str] = &["requires"];
const SERIES_FIELDS: &[FieldSpec] = &[
    field("name", FieldType::Text, true),
    field("ordered", FieldType::Boolean, true),
];
const SERIES_ENTRY_FIELDS: &[FieldSpec] = &[
    field("series_id", FieldType::Ref(Entity::Series), true),
    field("work_id", FieldType::Ref(Entity::Work), true),
    field("position", FieldType::Integer, false),
];
const WORK_RELATION_FIELDS: &[FieldSpec] = &[
    field("work_id", FieldType::Ref(Entity::Work), true),
    field("related_work_id", FieldType::Ref(Entity::Work), true),
    field("relation", FieldType::OneOf(WORK_RELATIONS), true),
];
const REVOCATION_FIELDS: &[FieldSpec] = &[
    field("device_id", FieldType::Text, true),
    field("device_name", FieldType::Text, true),
];
const DEVICE_FIELDS: &[FieldSpec] = &[
    field("public_key", FieldType::Text, true),
    field("name", FieldType::Text, true),
    field("member_id", FieldType::Ref(Entity::Member), false),
    field("trusted_until", FieldType::Text, false),
];
/// How a request ended: accepted or rejected by an administrator, or withdrawn by
/// whoever asked. The row without a status carries the asker's note (D73).
pub const REQUEST_STATUSES: &[&str] = &["accepted", "rejected", "withdrawn"];
const CHANGE_REQUEST_FIELDS: &[FieldSpec] = &[
    field("status", FieldType::OneOf(REQUEST_STATUSES), false),
    field("note", FieldType::Text, false),
];
/// Where a person is with a work (D68). «In progress» is named per kind in the UI.
pub const MARK_STATUSES: &[&str] = &["pending", "in_progress", "finished", "abandoned"];
const MARK_FIELDS: &[FieldSpec] = &[
    field("member_id", FieldType::Ref(Entity::Member), true),
    field("work_id", FieldType::Ref(Entity::Work), true),
    field("status", FieldType::OneOf(MARK_STATUSES), false),
    field("status_on", FieldType::Date, false),
    field("rating", FieldType::Rating, false),
    field("favorite", FieldType::Boolean, false),
    field("wished", FieldType::Boolean, false),
];
const COPY_FIELDS: &[FieldSpec] = &[
    field("edition_id", FieldType::Ref(Entity::Edition), true),
    field("owner_id", FieldType::Ref(Entity::Member), false),
    field("format", FieldType::OneOf(FORMATS), true),
    field("quantity", FieldType::Integer, true),
    field("location", FieldType::Text, false),
    field("acquired_on", FieldType::Date, false),
    // A video game copy's platform: it belongs to the copy, not the edition.
    field("platform", FieldType::Text, false),
    // A TV series copy's held seasons, as free text ("1-2", "1-3, 5").
    field("seasons", FieldType::Text, false),
];

impl Entity {
    pub fn table(self) -> &'static str {
        match self {
            Entity::Library => "libraries",
            Entity::Work => "works",
            Entity::Person => "people",
            Entity::WorkContributor => "work_contributors",
            Entity::Edition => "editions",
            Entity::BookDetails => "book_details",
            Entity::ComicDetails => "comic_details",
            Entity::BoardgameDetails => "boardgame_details",
            Entity::FilmDetails => "film_details",
            Entity::TvSeriesDetails => "tv_series_details",
            Entity::Member => "members",
            Entity::Copy => "copies",
            Entity::Publisher => "publishers",
            Entity::Tag => "tags",
            Entity::WorkTag => "work_tags",
            Entity::WorkGenre => "work_genres",
            Entity::Series => "series",
            Entity::SeriesEntry => "series_entries",
            Entity::WorkRelation => "work_relations",
            Entity::Revocation => "revocations",
            Entity::Device => "devices",
            Entity::ChangeRequest => "change_requests",
            Entity::Mark => "marks",
        }
    }

    /// The column that says which library a row belongs to. A library is its own.
    pub fn library_column(self) -> &'static str {
        match self {
            Entity::Library => "id",
            _ => "library_id",
        }
    }

    /// Every kind's details share their edition's id.
    pub fn id_column(self) -> &'static str {
        if DETAILS.contains(&self) {
            "edition_id"
        } else {
            "id"
        }
    }

    pub fn fields(self) -> &'static [FieldSpec] {
        match self {
            Entity::Library => LIBRARY_FIELDS,
            Entity::Work => WORK_FIELDS,
            Entity::Person => PERSON_FIELDS,
            Entity::WorkContributor => WORK_CONTRIBUTOR_FIELDS,
            Entity::Edition => EDITION_FIELDS,
            Entity::BookDetails => BOOK_DETAILS_FIELDS,
            Entity::ComicDetails => COMIC_DETAILS_FIELDS,
            Entity::BoardgameDetails => BOARDGAME_DETAILS_FIELDS,
            Entity::FilmDetails => FILM_DETAILS_FIELDS,
            Entity::TvSeriesDetails => TV_SERIES_DETAILS_FIELDS,
            Entity::Member => MEMBER_FIELDS,
            Entity::Copy => COPY_FIELDS,
            Entity::Publisher => PUBLISHER_FIELDS,
            Entity::Tag => TAG_FIELDS,
            Entity::WorkTag => WORK_TAG_FIELDS,
            Entity::WorkGenre => WORK_GENRE_FIELDS,
            Entity::Series => SERIES_FIELDS,
            Entity::SeriesEntry => SERIES_ENTRY_FIELDS,
            Entity::WorkRelation => WORK_RELATION_FIELDS,
            Entity::Revocation => REVOCATION_FIELDS,
            Entity::Device => DEVICE_FIELDS,
            Entity::ChangeRequest => CHANGE_REQUEST_FIELDS,
            Entity::Mark => MARK_FIELDS,
        }
    }

    pub fn field(self, name: &str) -> Option<&'static FieldSpec> {
        self.fields().iter().find(|spec| spec.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::{ContributorRole, Format, Kind};

    #[test]
    fn copy_formats_match_the_format_enum() {
        let Some(FieldSpec {
            field_type: FieldType::OneOf(names),
            ..
        }) = Entity::Copy.field("format")
        else {
            panic!("copy.format must be a OneOf field");
        };
        let enum_names: Vec<&str> = Format::ALL.into_iter().map(Format::as_str).collect();
        assert_eq!(*names, enum_names.as_slice());
    }

    #[test]
    fn kinds_match_the_kind_enum() {
        let enum_names: Vec<&str> = Kind::ALL.into_iter().map(Kind::as_str).collect();
        assert_eq!(KINDS, enum_names.as_slice());
    }

    #[test]
    fn contributor_roles_match_the_contributor_role_enum() {
        let enum_names: Vec<&str> = ContributorRole::ALL
            .into_iter()
            .map(ContributorRole::as_str)
            .collect();
        assert_eq!(CONTRIBUTOR_ROLES, enum_names.as_slice());
    }
}
