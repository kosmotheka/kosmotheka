//! One result a person can pick to prefill the add form. Field names match
//! `EditionInput` (crates/domain) where they exist, so the UI can copy them across.

use kosmotheka_domain::{ContributorRole, Format, Kind};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateContributor {
    pub name: String,
    pub role: ContributorRole,
}

/// The game an expansion needs (D61, «necesita»): kept on the candidate, though nothing creates the relation yet.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateExpands {
    pub external_id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub kind: Kind,
    pub source: super::Source,
    pub external_id: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub authors: Vec<String>,
    /// Contributors in any role other than author (`authors` means `author`).
    pub contributors: Vec<CandidateContributor>,
    pub publisher: Option<String>,
    pub published_year: Option<i32>,
    pub language: Option<String>,
    pub cover_url: Option<String>,
    /// ISBN-13.
    pub isbn: Option<String>,
    pub barcode: Option<String>,
    pub number: Option<String>,
    pub players_min: Option<i64>,
    pub players_max: Option<i64>,
    pub duration_minutes: Option<i64>,
    pub min_age: Option<i64>,
    pub seasons: Option<i64>,
    pub platform: Option<String>,
    /// A suggested copy format, when the provider knows it (e.g. `SingleIssue` for a comic).
    pub format: Option<Format>,
    /// The base game, when this one is an expansion of it (BoardGameGeek).
    pub expands: Option<CandidateExpands>,
    /// Set when this candidate's ISBN or barcode already exists in the active library.
    pub existing_edition_id: Option<String>,
}

impl Candidate {
    /// `Source` has no natural default (nothing to prefer), so build a
    /// `Candidate` from its required fields instead of deriving `Default`.
    pub fn new(kind: Kind, source: super::Source, title: impl Into<String>) -> Self {
        Self {
            kind,
            source,
            external_id: None,
            title: title.into(),
            subtitle: None,
            authors: Vec::new(),
            contributors: Vec::new(),
            publisher: None,
            published_year: None,
            language: None,
            cover_url: None,
            isbn: None,
            barcode: None,
            number: None,
            players_min: None,
            players_max: None,
            duration_minutes: None,
            min_age: None,
            seasons: None,
            platform: None,
            format: None,
            expands: None,
            existing_edition_id: None,
        }
    }
}
