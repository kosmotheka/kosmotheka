//! Wikidata (CC0, no key): the universal fallback for films, TV series, video
//! games and comics, and a second source for board games (D18). Three requests
//! at most: `wbsearchentities` for candidate ids, one `wbgetentities` for their
//! labels and claims (filtered by instance-of, P31), and one more `wbgetentities`
//! for the labels of the entity-valued claims (people, studios, platforms,
//! publishers) those candidates use. https://www.wikidata.org/w/api.php
//!
//! Class ids (P31) were checked by hand against real items before use; see the
//! `class ids` module doc below for what changed from a first guess.

use kosmotheka_domain::{ContributorRole, Kind};
use reqwest::StatusCode;
use serde_json::Value;

use super::{Candidate, CandidateContributor, ProviderError, Source, text, year_in};

const API: &str = "https://www.wikidata.org/w/api.php";

/// Acceptable P31 (instance of) values per kind. Verified against real items:
/// - film: a 2021 film (Dune, Q60834962) is P31 Q11424 directly.
/// - tv_series / videogame / comic / book: verified via each id's own Wikidata label.
/// - boardgame: Q1272194 is "tile-based game", not "tabletop game" as first
///   guessed. The real "tabletop game" class, confirmed as the direct P31 of a
///   real board game (Pandemic, Q531592), is Q3244175; used here instead.
///   Q142714 (card game) is kept: a real card game (Rummy, Q756011) uses it
///   directly, even though flashier examples (Magic: The Gathering, Skip-Bo)
///   use narrower subclasses this simple direct-P31 check does not reach.
/// - comic: many well-known comics are none of the general classes. Watchmen and Maus are «limited series»
///   (Q3297186), each Astérix album is a «comic book album» (Q2831984); «comic book» (Q1760610) and «comic strip»
///   (Q838795) complete them. Checked on 27/10/2026 against Watchmen, Maus, Blacksad, Astérix, Mortadelo y Filemón,
///   The Sandman and Persépolis.
fn classes_for(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::Film => &["Q11424", "Q24869", "Q506240"],
        Kind::TvSeries => &["Q5398426", "Q526877", "Q1259759"],
        Kind::Videogame => &["Q7889"],
        Kind::Boardgame => &["Q131436", "Q3244175", "Q142714"],
        Kind::Comic => &[
            "Q1004",
            "Q14406742",
            "Q8274",
            "Q21198342",
            "Q725377",
            "Q3297186",
            "Q2831984",
            "Q1760610",
            "Q838795",
        ],
        Kind::Book => &["Q7725634", "Q47461344", "Q571"],
        Kind::Music | Kind::Other => &[],
    }
}

/// One search result still missing the labels of the ids it references.
struct ItemFacts {
    candidate: Candidate,
    contributor_ids: Vec<(ContributorRole, String)>,
    publisher_id: Option<String>,
    platform_id: Option<String>,
    author_ids: Vec<String>,
}

impl ItemFacts {
    fn label_ids(&self) -> impl Iterator<Item = &str> {
        self.contributor_ids
            .iter()
            .map(|(_, id)| id.as_str())
            .chain(self.publisher_id.as_deref())
            .chain(self.platform_id.as_deref())
            .chain(self.author_ids.iter().map(String::as_str))
    }
}

pub async fn search(
    client: &reqwest::Client,
    kind: Kind,
    query: &str,
) -> Result<Vec<Candidate>, ProviderError> {
    let classes = classes_for(kind);
    if classes.is_empty() {
        return Ok(Vec::new());
    }

    let ids = search_ids(client, query).await?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let body = get_entities(client, &ids, "labels|claims", "es|en").await?;
    let mut items = parse_items(&body, &ids, kind, classes);
    items.truncate(10);
    if items.is_empty() {
        return Ok(Vec::new());
    }

    let mut label_ids: Vec<&str> = items.iter().flat_map(ItemFacts::label_ids).collect();
    label_ids.sort_unstable();
    label_ids.dedup();
    // The Wikidata API caps `ids` at 50; candidates rarely reference more
    // referenced people/organizations than that, so a plain truncate is fine.
    label_ids.truncate(50);

    if label_ids.is_empty() {
        return Ok(items.into_iter().map(|item| item.candidate).collect());
    }

    let owned_ids: Vec<String> = label_ids.into_iter().map(str::to_owned).collect();
    let label_body = get_entities(client, &owned_ids, "labels", "es|en").await?;
    Ok(items
        .into_iter()
        .map(|item| apply_labels(item, &label_body))
        .collect())
}

async fn search_ids(client: &reqwest::Client, query: &str) -> Result<Vec<String>, ProviderError> {
    let response = client
        .get(API)
        .query(&[
            ("action", "wbsearchentities"),
            ("search", query),
            ("language", "es"),
            ("uselang", "es"),
            ("type", "item"),
            ("limit", "15"),
            ("format", "json"),
        ])
        .send()
        .await?;
    let body = read_body(response).await?;
    Ok(parse_search_ids(&body))
}

async fn get_entities(
    client: &reqwest::Client,
    ids: &[String],
    props: &str,
    languages: &str,
) -> Result<Value, ProviderError> {
    let joined = ids.join("|");
    let response = client
        .get(API)
        .query(&[
            ("action", "wbgetentities"),
            ("ids", joined.as_str()),
            ("props", props),
            ("languages", languages),
            ("format", "json"),
        ])
        .send()
        .await?;
    read_body(response).await
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

/// Pure: reads the ids out of a `wbsearchentities` response, in relevance order.
fn parse_search_ids(body: &Value) -> Vec<String> {
    body.get("search")
        .and_then(Value::as_array)
        .map(|results| {
            results
                .iter()
                .filter_map(|r| r.get("id").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Pure: reads items out of a `wbgetentities` response, keeping only those whose
/// P31 matches one of `classes`, in the order given by `ids`, at most 10.
fn parse_items(body: &Value, ids: &[String], kind: Kind, classes: &[&str]) -> Vec<ItemFacts> {
    let Some(entities) = body.get("entities").and_then(Value::as_object) else {
        return Vec::new();
    };

    let mut items = Vec::new();
    for id in ids {
        if items.len() == 10 {
            break;
        }
        let Some(entity) = entities.get(id.as_str()) else {
            continue;
        };
        let claims = entity.get("claims").cloned().unwrap_or(Value::Null);
        let instance_of = entity_ids(&claims, "P31");
        if !instance_of.iter().any(|c| classes.contains(&c.as_str())) {
            continue;
        }
        let Some(title) = label_of(entity) else {
            continue;
        };

        let mut candidate = Candidate::new(kind, Source::Wikidata, title);
        candidate.external_id = Some(id.clone());
        candidate.cover_url = string_claim(&claims, "P18").map(|name| commons_file_path(&name));
        candidate.published_year =
            year_claim(&claims, "P577").or_else(|| year_claim(&claims, "P580"));

        let mut contributor_ids = Vec::new();
        let mut publisher_id = None;
        let mut platform_id = None;
        let mut author_ids = Vec::new();

        match kind {
            Kind::Film => {
                candidate.duration_minutes = duration_minutes(&claims);
                if let Some(director) = entity_id(&claims, "P57") {
                    contributor_ids.push((ContributorRole::Director, director));
                }
            }
            Kind::TvSeries => {
                candidate.seasons = quantity_int(&claims, "P2437");
            }
            Kind::Videogame => {
                if let Some(developer) = entity_id(&claims, "P178") {
                    contributor_ids.push((ContributorRole::Studio, developer));
                }
                publisher_id = entity_id(&claims, "P123");
                platform_id = entity_ids(&claims, "P400").into_iter().next();
            }
            Kind::Boardgame => {
                // Who designed a game is written three ways: «designed by» (Carcassonne), «creator» (Ticket to Ride)
                // or «developer» (Pandemic today). The first one the item has, with every name in it.
                let designers = ["P287", "P170", "P178"]
                    .into_iter()
                    .map(|property| entity_ids(&claims, property))
                    .find(|ids| !ids.is_empty())
                    .unwrap_or_default();
                for designer in designers {
                    contributor_ids.push((ContributorRole::Designer, designer));
                }
                publisher_id = entity_id(&claims, "P123");
                candidate.players_min = quantity_int(&claims, "P1872");
                candidate.players_max = quantity_int(&claims, "P1873");
            }
            Kind::Comic => {
                let writer = entity_id(&claims, "P50")
                    .or_else(|| entity_id(&claims, "P58"))
                    .or_else(|| entity_id(&claims, "P170"));
                if let Some(writer) = writer {
                    contributor_ids.push((ContributorRole::Writer, writer));
                }
                if let Some(illustrator) = entity_id(&claims, "P110") {
                    contributor_ids.push((ContributorRole::Illustrator, illustrator));
                }
                publisher_id = entity_id(&claims, "P123");
            }
            Kind::Book => {
                author_ids = entity_ids(&claims, "P50");
                publisher_id = entity_id(&claims, "P123");
            }
            Kind::Music | Kind::Other => {}
        }

        items.push(ItemFacts {
            candidate,
            contributor_ids,
            publisher_id,
            platform_id,
            author_ids,
        });
    }
    items
}

/// Pure: fills in the names of an item's contributors, publisher, platform and
/// authors from a second `wbgetentities` (labels-only) response.
fn apply_labels(item: ItemFacts, label_body: &Value) -> Candidate {
    let mut candidate = item.candidate;
    let entities = label_body.get("entities").and_then(Value::as_object);

    candidate.authors = item
        .author_ids
        .iter()
        .filter_map(|id| resolve_label(entities, id))
        .collect();
    candidate.contributors = item
        .contributor_ids
        .iter()
        .filter_map(|(role, id)| {
            resolve_label(entities, id).map(|name| CandidateContributor { name, role: *role })
        })
        .collect();
    candidate.publisher = item
        .publisher_id
        .as_deref()
        .and_then(|id| resolve_label(entities, id));
    candidate.platform = item
        .platform_id
        .as_deref()
        .and_then(|id| resolve_label(entities, id));

    candidate
}

fn resolve_label(entities: Option<&serde_json::Map<String, Value>>, id: &str) -> Option<String> {
    let labels = entities?.get(id)?.get("labels")?.as_object()?;
    labels
        .get("es")
        .or_else(|| labels.get("en"))
        .and_then(|l| l.get("value"))
        .and_then(Value::as_str)
        .and_then(|s| text(Some(s)))
}

fn label_of(entity: &Value) -> Option<String> {
    let labels = entity.get("labels")?;
    labels
        .get("es")
        .or_else(|| labels.get("en"))
        .and_then(|l| l.get("value"))
        .and_then(Value::as_str)
        .and_then(|s| text(Some(s)))
}

/// The values of a repeated claim, as the raw `datavalue.value` JSON.
fn claim_values<'a>(claims: &'a Value, prop: &str) -> Vec<&'a Value> {
    claims
        .get(prop)
        .and_then(Value::as_array)
        .map(|statements| {
            statements
                .iter()
                .filter_map(|s| s.pointer("/mainsnak/datavalue/value"))
                .collect()
        })
        .unwrap_or_default()
}

fn entity_ids(claims: &Value, prop: &str) -> Vec<String> {
    claim_values(claims, prop)
        .into_iter()
        .filter_map(|v| v.get("id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

fn entity_id(claims: &Value, prop: &str) -> Option<String> {
    entity_ids(claims, prop).into_iter().next()
}

fn string_claim(claims: &Value, prop: &str) -> Option<String> {
    claim_values(claims, prop)
        .into_iter()
        .next()
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// The first four digits of a Wikidata time value, e.g. `2017` from
/// `"+2017-03-03T00:00:00Z"`. Reuses `year_in`, so a leading `+`/`-` sign
/// (which is not a digit) is skipped like any other separator.
fn year_claim(claims: &Value, prop: &str) -> Option<i32> {
    let time = claim_values(claims, prop)
        .into_iter()
        .next()?
        .get("time")?
        .as_str()?;
    year_in(time)
}

/// A quantity claim's amount and unit id (e.g. `Q7727` for minute), when the
/// unit is a Wikidata item. Dimensionless quantities (players, seasons) carry
/// unit `"1"`, which is not an item id and comes back as `None`.
fn quantity(claims: &Value, prop: &str) -> Option<(i64, Option<String>)> {
    let value = claim_values(claims, prop).into_iter().next()?;
    let amount: i64 = value
        .get("amount")?
        .as_str()?
        .trim_start_matches('+')
        .parse()
        .ok()?;
    let unit = value
        .get("unit")
        .and_then(Value::as_str)
        .and_then(|u| u.rsplit('/').next())
        .filter(|u| u.starts_with('Q'))
        .map(str::to_owned);
    Some((amount, unit))
}

fn quantity_int(claims: &Value, prop: &str) -> Option<i64> {
    quantity(claims, prop).map(|(amount, _)| amount)
}

/// P2047 (duration), kept only when its unit is Q7727 (minute).
fn duration_minutes(claims: &Value) -> Option<i64> {
    let (amount, unit) = quantity(claims, "P2047")?;
    (unit.as_deref() == Some("Q7727")).then_some(amount)
}

/// A Commons `Special:FilePath` URL for a `P18` file name, which redirects
/// (through `commons.wikimedia.org` itself, then to `thumb.wikimedia.org`) to
/// the actual image. `COVER_HOSTS` only needs `commons.wikimedia.org`: the
/// http client's redirect policy already follows https redirects afterwards.
fn commons_file_path(file_name: &str) -> String {
    let underscored = file_name.replace(' ', "_");
    let mut url = reqwest::Url::parse("https://commons.wikimedia.org/wiki/Special:FilePath")
        .expect("static URL is valid");
    url.path_segments_mut()
        .expect("static URL can be a base")
        .push(&underscored);
    url.set_query(Some("width=500"));
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILM: &str = include_str!("../../tests/fixtures/wikidata-entities-film.json");
    const VIDEOGAME: &str = include_str!("../../tests/fixtures/wikidata-entities-videogame.json");
    const BOARDGAME: &str = include_str!("../../tests/fixtures/wikidata-entities-boardgame.json");
    const LABELS: &str = include_str!("../../tests/fixtures/wikidata-labels.json");
    const BOARDGAME_CREATORS: &str =
        include_str!("../../tests/fixtures/wikidata-entities-boardgame-creators.json");
    const COMICS: &str = include_str!("../../tests/fixtures/wikidata-entities-comics.json");

    #[test]
    fn reads_a_film_with_director_and_duration() {
        let body: Value = serde_json::from_str(FILM).unwrap();
        let ids = vec!["Q60834962".to_owned()];
        let items = parse_items(&body, &ids, Kind::Film, classes_for(Kind::Film));

        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item.candidate.title, "Dune");
        assert_eq!(item.candidate.external_id.as_deref(), Some("Q60834962"));
        assert_eq!(item.candidate.published_year, Some(2021));
        assert_eq!(item.candidate.duration_minutes, Some(155));
        assert_eq!(
            item.contributor_ids,
            vec![(ContributorRole::Director, "Q548823".to_owned())]
        );

        let labels: Value = serde_json::from_str(LABELS).unwrap();
        let candidate = apply_labels(
            ItemFacts {
                candidate: item.candidate.clone(),
                contributor_ids: item.contributor_ids.clone(),
                publisher_id: item.publisher_id.clone(),
                platform_id: item.platform_id.clone(),
                author_ids: item.author_ids.clone(),
            },
            &labels,
        );
        assert_eq!(
            candidate.contributors,
            vec![CandidateContributor {
                name: "Denis Villeneuve".to_owned(),
                role: ContributorRole::Director,
            }]
        );
    }

    #[test]
    fn reads_a_videogame_with_developer_and_platform() {
        let body: Value = serde_json::from_str(VIDEOGAME).unwrap();
        let ids = vec!["Q17185964".to_owned()];
        let items = parse_items(&body, &ids, Kind::Videogame, classes_for(Kind::Videogame));

        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(
            item.candidate.title,
            "The Legend of Zelda: Breath of the Wild"
        );
        assert_eq!(item.candidate.published_year, Some(2017));
        assert_eq!(item.publisher_id.as_deref(), Some("Q8093"));
        assert_eq!(item.platform_id.as_deref(), Some("Q56942"));
        assert_eq!(
            item.contributor_ids,
            vec![(ContributorRole::Studio, "Q20972825".to_owned())]
        );

        let labels: Value = serde_json::from_str(LABELS).unwrap();
        let candidate = apply_labels(
            ItemFacts {
                candidate: item.candidate.clone(),
                contributor_ids: item.contributor_ids.clone(),
                publisher_id: item.publisher_id.clone(),
                platform_id: item.platform_id.clone(),
                author_ids: item.author_ids.clone(),
            },
            &labels,
        );
        assert_eq!(candidate.publisher.as_deref(), Some("Nintendo"));
        assert_eq!(candidate.platform.as_deref(), Some("Nintendo Wii U"));
        assert_eq!(
            candidate.contributors,
            vec![CandidateContributor {
                name: "Nintendo Entertainment Planning & Development".to_owned(),
                role: ContributorRole::Studio,
            }]
        );
    }

    #[test]
    fn reads_a_boardgame_with_designer_and_players() {
        let body: Value = serde_json::from_str(BOARDGAME).unwrap();
        let ids = vec!["Q531592".to_owned()];
        let items = parse_items(&body, &ids, Kind::Boardgame, classes_for(Kind::Boardgame));

        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item.candidate.title, "Pandemic");
        assert_eq!(item.candidate.players_min, Some(2));
        assert_eq!(item.candidate.players_max, Some(4));
        assert_eq!(
            item.contributor_ids,
            vec![(ContributorRole::Designer, "Q24053471".to_owned())]
        );
        assert_eq!(
            item.candidate.cover_url.as_deref(),
            Some(
                "https://commons.wikimedia.org/wiki/Special:FilePath/Essen_2008_50101.jpg?width=500"
            )
        );

        let labels: Value = serde_json::from_str(LABELS).unwrap();
        let candidate = apply_labels(
            ItemFacts {
                candidate: item.candidate.clone(),
                contributor_ids: item.contributor_ids.clone(),
                publisher_id: item.publisher_id.clone(),
                platform_id: item.platform_id.clone(),
                author_ids: item.author_ids.clone(),
            },
            &labels,
        );
        assert_eq!(
            candidate.contributors,
            vec![CandidateContributor {
                name: "Matt Leacock".to_owned(),
                role: ContributorRole::Designer,
            }]
        );
    }

    #[test]
    fn a_boardgame_designer_comes_from_creator_when_designed_by_is_missing() {
        let body: Value = serde_json::from_str(BOARDGAME_CREATORS).unwrap();
        // Ticket to Ride has only «creator» (and «developer»); Carcassonne has «designed by».
        let ids = vec!["Q228308".to_owned(), "Q17262".to_owned()];
        let items = parse_items(&body, &ids, Kind::Boardgame, classes_for(Kind::Boardgame));

        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].contributor_ids,
            vec![(ContributorRole::Designer, "Q431402".to_owned())]
        );
        assert_eq!(
            items[1].contributor_ids,
            vec![(ContributorRole::Designer, "Q66734".to_owned())]
        );
    }

    #[test]
    fn a_limited_series_and_a_comic_album_are_comics() {
        let body: Value = serde_json::from_str(COMICS).unwrap();
        // Watchmen and an Astérix album: neither is a «comic» or a «comic book series» in Wikidata.
        let ids = vec!["Q128444".to_owned(), "Q618719".to_owned()];
        let items = parse_items(&body, &ids, Kind::Comic, classes_for(Kind::Comic));

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].candidate.title, "Watchmen");
        assert_eq!(
            items[0].contributor_ids,
            vec![(ContributorRole::Writer, "Q205739".to_owned())]
        );
        assert_eq!(items[1].candidate.title, "Astérix el Galo");
    }

    #[test]
    fn the_class_filter_drops_items_of_the_wrong_kind() {
        let body: Value = serde_json::from_str(FILM).unwrap();
        let ids = vec!["Q60834962".to_owned()];
        // The film fixture's P31 is Q11424, not one of the tv_series classes.
        let items = parse_items(&body, &ids, Kind::TvSeries, classes_for(Kind::TvSeries));
        assert!(items.is_empty());
    }

    #[test]
    fn commons_url_replaces_spaces_and_keeps_accents_encoded() {
        assert_eq!(
            commons_file_path("Osadníci z Katanu.jpg"),
            "https://commons.wikimedia.org/wiki/Special:FilePath/Osadn%C3%ADci_z_Katanu.jpg?width=500"
        );
    }
}
