//! The closed list of genres shipped with the app, per work kind (not rows of the
//! library: just keys). Mirrors nothing outside Rust; the UI translates the keys.

const BOOK: &[&str] = &[
    "novel",
    "science_fiction",
    "fantasy",
    "horror",
    "mystery",
    "historical",
    "romance",
    "adventure",
    "poetry",
    "theatre",
    "essay",
    "biography",
    "popular_science",
    "children",
    "young_adult",
    "roleplaying",
    "cooking",
    "art",
    "travel",
    "others",
];
const COMIC: &[&str] = &[
    "superheroes",
    "manga",
    "science_fiction",
    "fantasy",
    "horror",
    "humor",
    "adventure",
    "graphic_novel",
    "crime",
    "historical",
    "children",
    "others",
];
const BOARDGAME: &[&str] = &[
    "strategy",
    "family",
    "party",
    "cooperative",
    "abstract",
    "card_game",
    "tile_placement",
    "worker_placement",
    "deck_building",
    "deduction",
    "wargame",
    "children",
    "roleplaying",
    "others",
];
const FILM: &[&str] = &[
    "action",
    "adventure",
    "animation",
    "science_fiction",
    "comedy",
    "documentary",
    "drama",
    "fantasy",
    "musical",
    "thriller",
    "horror",
    "romance",
    "war",
    "western",
    "crime",
    "children",
    "others",
];
const VIDEOGAME: &[&str] = &[
    "action",
    "adventure",
    "roleplaying",
    "strategy",
    "platformer",
    "puzzle",
    "racing",
    "sports",
    "fighting",
    "shooter",
    "simulation",
    "horror",
    "party",
    "others",
];
const MUSIC: &[&str] = &[
    "rock",
    "pop",
    "jazz",
    "classical",
    "electronic",
    "hip_hop",
    "folk",
    "flamenco",
    "blues",
    "soul",
    "metal",
    "soundtrack",
    "children",
    "others",
];
const OTHER: &[&str] = &[];

/// Every genre of every kind, each once, in `genres_of` order per kind.
pub const GENRES: &[&str] = &[
    "novel",
    "science_fiction",
    "fantasy",
    "horror",
    "mystery",
    "historical",
    "romance",
    "adventure",
    "poetry",
    "theatre",
    "essay",
    "biography",
    "popular_science",
    "children",
    "young_adult",
    "roleplaying",
    "cooking",
    "art",
    "travel",
    "others",
    "superheroes",
    "manga",
    "humor",
    "graphic_novel",
    "crime",
    "strategy",
    "family",
    "party",
    "cooperative",
    "abstract",
    "card_game",
    "tile_placement",
    "worker_placement",
    "deck_building",
    "deduction",
    "wargame",
    "action",
    "animation",
    "comedy",
    "documentary",
    "drama",
    "musical",
    "thriller",
    "war",
    "western",
    "platformer",
    "puzzle",
    "racing",
    "sports",
    "fighting",
    "shooter",
    "simulation",
    "rock",
    "pop",
    "jazz",
    "classical",
    "electronic",
    "hip_hop",
    "folk",
    "flamenco",
    "blues",
    "soul",
    "metal",
    "soundtrack",
];

/// The genres a kind offers, in the order the UI lists them.
pub fn genres_of(kind: &str) -> &'static [&'static str] {
    match kind {
        "book" => BOOK,
        "comic" => COMIC,
        "boardgame" => BOARDGAME,
        "film" | "tv_series" => FILM,
        "videogame" => VIDEOGAME,
        "music" => MUSIC,
        _ => OTHER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::KINDS;
    use std::collections::HashSet;

    #[test]
    fn every_key_of_every_kind_is_in_genres() {
        for kind in KINDS {
            for genre in genres_of(kind) {
                assert!(
                    GENRES.contains(genre),
                    "{genre} of {kind} is missing from GENRES"
                );
            }
        }
    }

    #[test]
    fn no_kind_has_duplicates() {
        for kind in KINDS {
            let list = genres_of(kind);
            let set: HashSet<&&str> = list.iter().collect();
            assert_eq!(set.len(), list.len(), "{kind} has duplicate genres");
        }
    }

    #[test]
    fn other_has_none() {
        assert!(genres_of("other").is_empty());
    }

    #[test]
    fn the_spec_lists_the_same_genres() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../spec/operation.schema.json")).unwrap();
        let mut found = None;
        find_genre_enum(&schema, &mut found);
        let spec: Vec<&str> = found
            .expect("work_genre.genre enum in the spec")
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(spec, GENRES);
    }

    fn find_genre_enum<'a>(
        value: &'a serde_json::Value,
        found: &mut Option<&'a Vec<serde_json::Value>>,
    ) {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(list) = map
                    .get("genre")
                    .and_then(|g| g.get("enum"))
                    .and_then(|e| e.as_array())
                {
                    *found = Some(list);
                }
                map.values().for_each(|v| find_genre_enum(v, found));
            }
            serde_json::Value::Array(items) => items.iter().for_each(|v| find_genre_enum(v, found)),
            _ => {}
        }
    }
}
