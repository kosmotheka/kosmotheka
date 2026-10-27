//! Whether a newer version was published (D104, first step): the computer asks GitHub for the latest release, at
//! most once a day, and the screen points to the downloads page. Phones and the snap are kept up to date by their
//! store, so they never ask.

use serde::Deserialize;

use crate::error::Result;
use crate::whats_new::parse_version;

/// Only published releases: a draft or a pre-release is never «latest», so nobody hears of a version before the
/// maintainer publishes it.
const LATEST: &str = "https://api.github.com/repos/kosmotheka/kosmotheka/releases/latest";

const DAY_SECONDS: u64 = 24 * 60 * 60;

/// The version running now, the one the answer is compared with.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// Google Play on the phone and the Snap Store on Linux update the app themselves.
pub fn handled_by_a_store() -> bool {
    cfg!(mobile) || std::env::var_os("SNAP").is_some()
}

/// A day since the last time, or never asked. A clock set back does not stop it for good.
pub fn due(checked_at: Option<u64>, now: u64) -> bool {
    checked_at.is_none_or(|at| at > now || now - at >= DAY_SECONDS)
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

/// The version of the latest release, from GitHub's answer: tags are written `v0.7.0`.
pub fn latest_version(body: &str) -> Option<String> {
    let release: Release = serde_json::from_str(body).ok()?;
    let version = release.tag_name.trim_start_matches('v');
    parse_version(version).map(|_| version.to_owned())
}

/// Only a higher number: a build ahead of the published one (a test build) hears nothing.
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(latest), Some(current)) => latest > current,
        _ => false,
    }
}

/// The latest published version, whatever this one is.
pub async fn latest(client: &reqwest::Client) -> Result<Option<String>> {
    let body = client
        .get(LATEST)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(reqwest::Error::without_url)?
        .text()
        .await
        .map_err(reqwest::Error::without_url)?;
    Ok(latest_version(&body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_comes_from_the_tag_of_githubs_answer() {
        let body = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/github-latest-release.json"
        ))
        .unwrap();
        assert_eq!(latest_version(&body).as_deref(), Some("0.7.0"));
        assert_eq!(latest_version(r#"{"tag_name":"nightly"}"#), None);
        assert_eq!(latest_version("Not Found"), None);
    }

    #[test]
    fn only_a_higher_number_is_news() {
        assert!(is_newer("0.8.0", "0.7.0"));
        assert!(is_newer("0.7.1", "0.7.0"));
        assert!(is_newer("1.0.0", "0.10.3"));
        assert!(!is_newer("0.7.0", "0.7.0"));
        assert!(!is_newer("0.7.0", "0.8.0"));
        assert!(!is_newer("0.8", "0.7.0"));
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_github_says_the_latest_version() {
        let client = crate::metadata::http_client().unwrap();
        let latest = latest(&client).await.unwrap();
        println!("latest: {latest:?}");
        assert!(latest.is_some());
    }

    #[test]
    fn it_asks_at_most_once_a_day() {
        let now = 1_800_000_000;
        assert!(due(None, now));
        assert!(!due(Some(now - 60), now));
        assert!(!due(Some(now - DAY_SECONDS + 1), now));
        assert!(due(Some(now - DAY_SECONDS), now));
        // The clock went back: ask, instead of waiting until the date comes round again.
        assert!(due(Some(now + 3600), now));
    }
}
