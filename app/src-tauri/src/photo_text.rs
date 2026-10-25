//! Reading text in a photo, on the device: the text recognition plugin (ML Kit on Android, no network) reads it, and
//! the domain decides what is worth keeping. Two uses: the ISBN printed on a book, for when its barcode is a shop's
//! label, where only what the check digit confirms is kept; and the spines of a shelf, where the lines worth searching
//! for in the catalogues are kept.

#[cfg(any(mobile, test))]
use kosmotheka_domain::Isbn;
#[cfg(mobile)]
use kosmotheka_domain::shelf::{self, ReadBlock, ReadLine};
use tauri::{AppHandle, Runtime};

use crate::error::{Error, Result};

/// The ISBNs found in `text`, in the order they appear, as 13 digits.
#[cfg(any(mobile, test))]
fn isbns_in(text: &str) -> Vec<String> {
    Isbn::find_in_text(text)
        .into_iter()
        .map(|isbn| isbn.as_str().to_owned())
        .collect()
}

#[cfg(mobile)]
async fn read<R: Runtime>(
    app: &AppHandle<R>,
    photo: &[u8],
) -> Result<tauri_plugin_text_recognition::RecognizedText> {
    use tauri::Manager;
    use tauri_plugin_text_recognition::{RecognizeTextRequest, TextRecognitionExt};

    let failed = |error: &dyn std::fmt::Display| Error::TextRecognition(error.to_string());
    let dir = app.path().app_cache_dir().map_err(|e| failed(&e))?;
    std::fs::create_dir_all(&dir)?;
    // Always the same file, removed once read: the photo is not kept.
    let path = dir.join("photo-text.jpg");
    std::fs::write(&path, photo)?;
    let read = app
        .text_recognition()
        .recognize_text(RecognizeTextRequest {
            path: path.to_string_lossy().into_owned(),
        })
        .await;
    let _ = std::fs::remove_file(&path);
    read.map_err(|e| failed(&e))
}

#[cfg(mobile)]
pub async fn isbns_in_photo<R: Runtime>(app: &AppHandle<R>, photo: &[u8]) -> Result<Vec<String>> {
    Ok(isbns_in(&read(app, photo).await?.text))
}

/// What to search for in the catalogues, from a photo of a shelf (`shelf::queries`).
#[cfg(mobile)]
pub async fn shelf_queries<R: Runtime>(app: &AppHandle<R>, photo: &[u8]) -> Result<Vec<String>> {
    let blocks: Vec<ReadBlock> = read(app, photo)
        .await?
        .blocks
        .into_iter()
        .map(|block| ReadBlock {
            lines: block
                .lines
                .into_iter()
                .map(|line| ReadLine {
                    text: line.text,
                    confidence: line.confidence,
                })
                .collect(),
        })
        .collect();
    Ok(shelf::queries(&blocks))
}

#[cfg(not(mobile))]
pub async fn isbns_in_photo<R: Runtime>(_app: &AppHandle<R>, _photo: &[u8]) -> Result<Vec<String>> {
    Err(Error::TextRecognition("not on this device".into()))
}

#[cfg(not(mobile))]
pub async fn shelf_queries<R: Runtime>(_app: &AppHandle<R>, _photo: &[u8]) -> Result<Vec<String>> {
    Err(Error::TextRecognition("not on this device".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_isbns_come_back_as_thirteen_digits() {
        assert_eq!(
            isbns_in("Ragnarok\nCOPIA DIGITAL INCLUIDA\n9\" 788469 \" 767016\nISBN 84-376-0494-X"),
            ["9788469767016", "9788437604947"]
        );
    }
}
