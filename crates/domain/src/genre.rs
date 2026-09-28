//! The closed list of genres shipped with the app, per work kind (not rows of the
//! library: just keys). Mirrors nothing outside Rust; the UI translates the keys.

const BOOK: &[&str] = &[
    "fantasy",
    "science_fiction",
    "horror",
    "mystery",
    "thriller",
    "romance",
    "historical_fiction",
    "adventure",
    "humor",
    "poetry",
    "theatre",
    "essay",
    "biography",
    "history",
    "science",
    "philosophy",
    "art",
    "cooking",
    "travel",
    "roleplaying",
    "children",
    "young_adult",
];
const COMIC: &[&str] = &[
    "superheroes",
    "manga",
    "fantasy",
    "science_fiction",
    "horror",
    "humor",
    "adventure",
    "crime",
    "historical_fiction",
    "slice_of_life",
    "children",
];
const BOARDGAME: &[&str] = &[
    "strategy",
    "family",
    "party",
    "cooperative",
    "abstract",
    "deduction",
    "wargame",
    "card_game",
    "dexterity",
    "trivia",
    "roleplaying",
    "children",
];
const FILM: &[&str] = &[
    "action",
    "adventure",
    "animation",
    "comedy",
    "drama",
    "fantasy",
    "science_fiction",
    "horror",
    "thriller",
    "romance",
    "crime",
    "war",
    "western",
    "musical",
    "documentary",
    "children",
];
const VIDEOGAME: &[&str] = &[
    "action",
    "adventure",
    "roleplaying",
    "strategy",
    "simulation",
    "sports",
    "racing",
    "puzzle",
    "platformer",
    "shooter",
    "fighting",
    "horror",
    "party",
    "children",
];
const MUSIC: &[&str] = &[
    "rock",
    "pop",
    "jazz",
    "classical",
    "electronic",
    "hip_hop",
    "folk",
    "metal",
    "blues",
    "soul",
    "reggae",
    "latin",
    "flamenco",
    "soundtrack",
    "world",
    "children",
];
const OTHER: &[&str] = &[];

/// Every genre of every kind, each once, in `genres_of` order per kind.
pub const GENRES: &[&str] = &[
    "fantasy",
    "science_fiction",
    "horror",
    "mystery",
    "thriller",
    "romance",
    "historical_fiction",
    "adventure",
    "humor",
    "poetry",
    "theatre",
    "essay",
    "biography",
    "history",
    "science",
    "philosophy",
    "art",
    "cooking",
    "travel",
    "roleplaying",
    "children",
    "young_adult",
    "superheroes",
    "manga",
    "crime",
    "slice_of_life",
    "strategy",
    "family",
    "party",
    "cooperative",
    "abstract",
    "deduction",
    "wargame",
    "card_game",
    "dexterity",
    "trivia",
    "action",
    "animation",
    "comedy",
    "drama",
    "war",
    "western",
    "musical",
    "documentary",
    "simulation",
    "sports",
    "racing",
    "puzzle",
    "platformer",
    "shooter",
    "fighting",
    "rock",
    "pop",
    "jazz",
    "classical",
    "electronic",
    "hip_hop",
    "folk",
    "metal",
    "blues",
    "soul",
    "reggae",
    "latin",
    "flamenco",
    "soundtrack",
    "world",
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
}
