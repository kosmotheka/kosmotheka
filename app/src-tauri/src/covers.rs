//! Cover images stored as local files named by the sha256 of their bytes (D27).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use reqwest::header::CONTENT_TYPE;
use sha2::{Digest, Sha256};
use url::Url;

use crate::error::{Error, Result};

pub const MAX_COVER_BYTES: usize = 5 * 1024 * 1024;

pub struct Covers {
    dir: PathBuf,
}

impl Covers {
    pub fn new(dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Downloads an already validated URL (see `library_domain::parse_cover_url`)
    /// and returns the stored file name.
    pub async fn download(&self, client: &reqwest::Client, url: &Url) -> Result<String> {
        let response = client.get(url.clone()).send().await?.error_for_status()?;
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        let extension = extension_for(&content_type).ok_or(Error::CoverRejected("not an image"))?;
        if response
            .content_length()
            .is_some_and(|length| length as usize > MAX_COVER_BYTES)
        {
            return Err(Error::CoverRejected("too large"));
        }
        let bytes = response.bytes().await?;
        if bytes.len() > MAX_COVER_BYTES {
            return Err(Error::CoverRejected("too large"));
        }
        self.store(&bytes, extension)
    }

    /// Same bytes, same name: storing a cover twice is harmless.
    pub fn store(&self, bytes: &[u8], extension: &str) -> Result<String> {
        let name = format!("{}.{extension}", sha256_hex(bytes));
        let path = self.dir.join(&name);
        if !path.exists() {
            // Write to a temporary file first so a crash never leaves half an image.
            let temporary = self.dir.join(format!(".{name}.part"));
            std::fs::write(&temporary, bytes)?;
            std::fs::rename(&temporary, &path)?;
        }
        Ok(name)
    }
}

fn extension_for(content_type: &str) -> Option<&'static str> {
    match content_type.split(';').next()?.trim() {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        _ => None,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("library-covers-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn stores_by_content_hash_once() {
        let covers = Covers::new(temp_dir("store")).unwrap();

        let first = covers.store(b"image bytes", "jpg").unwrap();
        let second = covers.store(b"image bytes", "jpg").unwrap();

        assert_eq!(first, second);
        assert_eq!(first.len(), 64 + 4);
        assert!(covers.dir().join(&first).exists());
        assert_eq!(
            std::fs::read_dir(covers.dir()).unwrap().count(),
            1,
            "no temporary files left"
        );
    }

    #[test]
    fn known_image_types_only() {
        assert_eq!(extension_for("image/jpeg"), Some("jpg"));
        assert_eq!(extension_for("image/png; charset=binary"), Some("png"));
        assert_eq!(extension_for("text/html"), None);
        assert_eq!(extension_for(""), None);
    }

    #[test]
    fn sha256_matches_a_known_value() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
