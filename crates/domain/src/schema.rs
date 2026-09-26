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
    Member,
    Copy,
    Publisher,
    Wish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Text,
    Integer,
    /// `YYYY-MM-DD`.
    Date,
    OneOf(&'static [&'static str]),
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
    "audiobook",
    "other",
];

// Declared as constants so the slices live for the whole program ('static).
const WORK_FIELDS: &[FieldSpec] = &[
    field("kind", FieldType::OneOf(&["book"]), true),
    field("title", FieldType::Text, true),
];
const LIBRARY_FIELDS: &[FieldSpec] = &[field("name", FieldType::Text, true)];
const PERSON_FIELDS: &[FieldSpec] = &[field("name", FieldType::Text, true)];
const WORK_CONTRIBUTOR_FIELDS: &[FieldSpec] = &[
    field("work_id", FieldType::Ref(Entity::Work), true),
    field("person_id", FieldType::Ref(Entity::Person), true),
    field("role", FieldType::OneOf(&["author"]), true),
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
];
const BOOK_DETAILS_FIELDS: &[FieldSpec] = &[
    field("isbn13", FieldType::Text, false),
    field("google_books_id", FieldType::Text, false),
    field("open_library_id", FieldType::Text, false),
];
const MEMBER_FIELDS: &[FieldSpec] = &[field("name", FieldType::Text, true)];
const PUBLISHER_FIELDS: &[FieldSpec] = &[field("name", FieldType::Text, true)];
const WISH_FIELDS: &[FieldSpec] = &[
    field("title", FieldType::Text, true),
    field("author", FieldType::Text, false),
    field("isbn13", FieldType::Text, false),
    field("note", FieldType::Text, false),
    field("cover_url", FieldType::Text, false),
];
const COPY_FIELDS: &[FieldSpec] = &[
    field("edition_id", FieldType::Ref(Entity::Edition), true),
    field("owner_id", FieldType::Ref(Entity::Member), false),
    field("format", FieldType::OneOf(FORMATS), true),
    field("quantity", FieldType::Integer, true),
    field("location", FieldType::Text, false),
    field("acquired_on", FieldType::Date, false),
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
            Entity::Member => "members",
            Entity::Copy => "copies",
            Entity::Publisher => "publishers",
            Entity::Wish => "wishes",
        }
    }

    /// Book details share their edition's id.
    pub fn id_column(self) -> &'static str {
        match self {
            Entity::BookDetails => "edition_id",
            _ => "id",
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
            Entity::Member => MEMBER_FIELDS,
            Entity::Copy => COPY_FIELDS,
            Entity::Publisher => PUBLISHER_FIELDS,
            Entity::Wish => WISH_FIELDS,
        }
    }

    pub fn field(self, name: &str) -> Option<&'static FieldSpec> {
        self.fields().iter().find(|spec| spec.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::Format;

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
}
