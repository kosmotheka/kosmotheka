//! Reading the ISBN printed on a book from a photo, for when its barcode is a shop's label: the text recognition
//! plugin reads the photo on the device (ML Kit, no network), and the domain keeps only what the check digit
//! confirms.

#[cfg(any(mobile, test))]
use kosmotheka_domain::Isbn;
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
pub async fn isbns_in_photo<R: Runtime>(app: &AppHandle<R>, photo: &[u8]) -> Result<Vec<String>> {
    use tauri::Manager;
    use tauri_plugin_text_recognition::{RecognizeTextRequest, TextRecognitionExt};

    let failed = |error: &dyn std::fmt::Display| Error::TextRecognition(error.to_string());
    let dir = app.path().app_cache_dir().map_err(|e| failed(&e))?;
    std::fs::create_dir_all(&dir)?;
    // Always the same file, removed once read: the photo is not kept.
    let path = dir.join("printed-isbn.jpg");
    std::fs::write(&path, photo)?;
    let read = app
        .text_recognition()
        .recognize_text(RecognizeTextRequest {
            path: path.to_string_lossy().into_owned(),
        })
        .await;
    let _ = std::fs::remove_file(&path);
    Ok(isbns_in(&read.map_err(|e| failed(&e))?.text))
}

#[cfg(not(mobile))]
pub async fn isbns_in_photo<R: Runtime>(_app: &AppHandle<R>, _photo: &[u8]) -> Result<Vec<String>> {
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
