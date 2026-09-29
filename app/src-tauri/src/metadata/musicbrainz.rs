//! MusicBrainz (musicbrainz.org): free, keyless, CC0 core release data, plus
//! its sibling Cover Art Archive for covers. https://musicbrainz.org/doc/MusicBrainz_API
//!
//! Search by barcode and by title share the same `/release/` endpoint and
//! response shape, so both go through `parse_releases`.

use std::collections::HashSet;

use kosmotheka_domain::{ContributorRole, Format, Kind};
use reqwest::StatusCode;
use serde_json::Value;

use super::{Candidate, CandidateContributor, ProviderError, Source, text, year_in};

const BASE: &str = "https://musicbrainz.org/ws/2";

/// Title search, for the add-by-search flow (D26).
pub async fn search(
    client: &reqwest::Client,
    query: &str,
) -> Result<Vec<Candidate>, ProviderError> {
    let response = client
        .get(format!("{BASE}/release/"))
        .query(&[("query", query), ("fmt", "json"), ("limit", "10")])
        .send()
        .await?;
    let body = read_body(response).await?;
    Ok(parse_releases(&body))
}

/// A scanned barcode (UPC/EAN): may match several releases (repress, region).
pub async fn lookup_barcode(
    client: &reqwest::Client,
    code: &str,
) -> Result<Vec<Candidate>, ProviderError> {
    let query = format!("barcode:{code}");
    let response = client
        .get(format!("{BASE}/release/"))
        .query(&[("query", query.as_str()), ("fmt", "json"), ("limit", "5")])
        .send()
        .await?;
    let body = read_body(response).await?;
    Ok(parse_releases(&body))
}

async fn read_body(response: reqwest::Response) -> Result<Value, ProviderError> {
    match response.status() {
        StatusCode::TOO_MANY_REQUESTS => return Err(ProviderError::Quota),
        status if !status.is_success() => {
            return Err(ProviderError::Unexpected(format!("HTTP {status}")));
        }
        _ => {}
    }
    Ok(response.json().await?)
}

pub fn parse_releases(body: &Value) -> Vec<Candidate> {
    let candidates = body
        .get("releases")
        .and_then(Value::as_array)
        .map(|releases| releases.iter().map(candidate_from_release).collect())
        .unwrap_or_default();
    dedupe(candidates)
}

/// Same release turns up under several repress/region entries; keep the first.
fn dedupe(candidates: Vec<Candidate>) -> Vec<Candidate> {
    let mut seen = HashSet::new();
    candidates
        .into_iter()
        .filter(|candidate| {
            let key = (
                candidate.title.to_lowercase(),
                candidate
                    .contributors
                    .first()
                    .map(|c| c.name.to_lowercase()),
                candidate.published_year,
            );
            seen.insert(key)
        })
        .collect()
}

fn candidate_from_release(release: &Value) -> Candidate {
    let str_field = |name: &str| text(release.get(name).and_then(Value::as_str));

    let mut candidate = Candidate::new(
        Kind::Music,
        Source::MusicBrainz,
        str_field("title").unwrap_or_default(),
    );
    candidate.contributors = release
        .get("artist-credit")
        .and_then(Value::as_array)
        .map(|artists| {
            artists
                .iter()
                .filter_map(|a| text(a.get("name").and_then(Value::as_str)))
                .map(|name| CandidateContributor {
                    name,
                    role: ContributorRole::Artist,
                })
                .collect()
        })
        .unwrap_or_default();
    candidate.publisher = release
        .pointer("/label-info/0/label/name")
        .and_then(Value::as_str)
        .and_then(|s| text(Some(s)));
    candidate.published_year = str_field("date").as_deref().and_then(year_in);
    candidate.barcode = str_field("barcode");
    candidate.external_id = str_field("id");
    candidate.cover_url = has_front_cover(release).then(|| {
        format!(
            "https://coverartarchive.org/release/{}/front-500",
            release
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
        )
    });
    candidate.format = release
        .pointer("/media/0/format")
        .and_then(Value::as_str)
        .and_then(format_from_medium);
    candidate
}

/// A release search result never carries `cover-art-archive` today (checked
/// against the live API on 2026-09-29), so a missing object means "unknown",
/// not "no cover" — the URL is still worth trying.
fn has_front_cover(release: &Value) -> bool {
    release
        .get("cover-art-archive")
        .and_then(|c| c.get("front"))
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn format_from_medium(format: &str) -> Option<Format> {
    match format {
        "CD" => Some(Format::Cd),
        "Vinyl" | "12\" Vinyl" | "7\" Vinyl" => Some(Format::Vinyl),
        "Cassette" => Some(Format::Cassette),
        "Digital Media" => Some(Format::Digital),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real response for query=barcode:628261245628, recorded on 2026-09-29.
    const BARCODE: &str =
        include_str!("../../tests/fixtures/musicbrainz-barcode-628261245628.json");
    const BARCODE_NOT_FOUND: &str =
        include_str!("../../tests/fixtures/musicbrainz-barcode-not-found.json");
    // Trimmed to 3 releases, recorded on 2026-09-29; the third duplicates the
    // first (same title, artist and year) to exercise dedup.
    const SEARCH: &str =
        include_str!("../../tests/fixtures/musicbrainz-search-random-access-memories.json");

    #[test]
    fn reads_a_real_barcode_match() {
        let candidates = parse_releases(&serde_json::from_str(BARCODE).unwrap());

        assert_eq!(candidates.len(), 1);
        let release = &candidates[0];
        assert_eq!(release.kind, Kind::Music);
        assert_eq!(release.source, Source::MusicBrainz);
        assert_eq!(release.title, "Miles Davis");
        assert_eq!(
            release.contributors,
            vec![CandidateContributor {
                name: "Miles Davis".into(),
                role: ContributorRole::Artist,
            }]
        );
        assert_eq!(release.publisher.as_deref(), Some("Madacy Entertainment"));
        assert_eq!(release.published_year, Some(2006));
        assert_eq!(release.barcode.as_deref(), Some("628261245628"));
        assert_eq!(
            release.external_id.as_deref(),
            Some("b485a424-9fbd-4603-8ae3-d00096b294ac")
        );
        assert_eq!(
            release.cover_url.as_deref(),
            Some(
                "https://coverartarchive.org/release/b485a424-9fbd-4603-8ae3-d00096b294ac/front-500"
            )
        );
        assert_eq!(release.format, Some(Format::Cd));
    }

    #[test]
    fn an_unknown_barcode_is_empty() {
        let candidates = parse_releases(&serde_json::from_str(BARCODE_NOT_FOUND).unwrap());
        assert!(candidates.is_empty());
    }

    #[test]
    fn a_search_dedupes_same_title_artist_and_year() {
        let candidates = parse_releases(&serde_json::from_str(SEARCH).unwrap());

        assert_eq!(
            candidates.len(),
            2,
            "the third release duplicates the first"
        );
        assert_eq!(candidates[0].published_year, Some(2023));
        assert_eq!(candidates[1].published_year, Some(2013));
        assert!(
            candidates
                .iter()
                .all(|c| c.title == "Random Access Memories")
        );
    }

    #[test]
    fn format_is_mapped_from_the_first_medium() {
        assert_eq!(format_from_medium("CD"), Some(Format::Cd));
        assert_eq!(format_from_medium("Vinyl"), Some(Format::Vinyl));
        assert_eq!(format_from_medium("12\" Vinyl"), Some(Format::Vinyl));
        assert_eq!(format_from_medium("7\" Vinyl"), Some(Format::Vinyl));
        assert_eq!(format_from_medium("Cassette"), Some(Format::Cassette));
        assert_eq!(format_from_medium("Digital Media"), Some(Format::Digital));
        assert_eq!(format_from_medium("SACD"), None);
    }

    #[test]
    fn a_missing_cover_art_archive_object_is_assumed_to_have_art() {
        let release = serde_json::json!({ "id": "abc", "title": "T" });
        assert!(has_front_cover(&release));
    }

    #[test]
    fn front_false_means_no_cover() {
        let release = serde_json::json!({
            "id": "abc",
            "title": "T",
            "cover-art-archive": { "front": false }
        });
        assert!(!has_front_cover(&release));
        assert_eq!(candidate_from_release(&release).cover_url, None);
    }

    #[test]
    fn a_search_with_no_releases_is_empty() {
        assert!(parse_releases(&serde_json::json!({})).is_empty());
    }
}
