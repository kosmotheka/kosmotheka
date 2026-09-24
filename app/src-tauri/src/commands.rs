use library_domain::{CopyInput, EditionInput, FieldError, Isbn, NewBook};
use serde::Serialize;
use tauri::State;
use url::Url;

use crate::catalog::{self, BookDetail, BookSummary, CreatedBook, HistoryEntry};
use crate::covers::Covers;
use crate::db::Database;
use crate::error::{Error, Result};
use crate::metadata::{Lookup, LookupResult};
use crate::settings::{self, Settings};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Saved<T> {
    #[serde(flatten)]
    pub value: T,
    /// The book was saved, but its cover could not be downloaded.
    pub cover_failed: bool,
}

#[tauri::command]
pub async fn create_book(
    database: State<'_, Database>,
    covers: State<'_, Covers>,
    lookup: State<'_, Lookup>,
    book: NewBook,
) -> Result<Saved<CreatedBook>> {
    let book = book.validate().map_err(Error::Validation)?;
    // Download before locking: the database lock must never be held while waiting on the network.
    let (cover, cover_failed) =
        download_cover(&covers, &lookup, book.edition.cover_url.as_ref()).await;
    let mut conn = database.lock()?;
    let created = catalog::create_book(&mut conn, &book, cover.as_deref())?;
    Ok(Saved {
        value: created,
        cover_failed,
    })
}

#[tauri::command]
pub async fn update_edition(
    database: State<'_, Database>,
    covers: State<'_, Covers>,
    lookup: State<'_, Lookup>,
    edition_id: String,
    edition: EditionInput,
) -> Result<Saved<()>> {
    let edition = edition.validate().map_err(Error::Validation)?;
    let (cover, cover_failed) = download_cover(&covers, &lookup, edition.cover_url.as_ref()).await;
    let mut conn = database.lock()?;
    catalog::update_edition(&mut conn, &edition_id, &edition, cover.as_deref())?;
    Ok(Saved {
        value: (),
        cover_failed,
    })
}

/// Returns the stored file name, or whether a download was attempted and failed.
async fn download_cover(
    covers: &Covers,
    lookup: &Lookup,
    url: Option<&Url>,
) -> (Option<String>, bool) {
    match url {
        None => (None, false),
        Some(url) => match covers.download(lookup.client(), url).await {
            Ok(name) => (Some(name), false),
            Err(_) => (None, true),
        },
    }
}

#[tauri::command]
pub async fn lookup_isbn(
    database: State<'_, Database>,
    lookup: State<'_, Lookup>,
    isbn: String,
) -> Result<LookupResult> {
    let isbn = Isbn::parse(&isbn)
        .map_err(|error| Error::Validation(vec![FieldError::new("isbn", error.into())]))?;
    // The block ends before `.await`, releasing the lock. Holding a MutexGuard across
    // an await would not even compile here: the guard cannot move between threads.
    let (api_key, existing_edition_id) = {
        let conn = database.lock()?;
        (
            settings::load(&conn)?.google_books_api_key,
            catalog::edition_with_isbn(&conn, isbn.as_str())?,
        )
    };
    let mut result = lookup.run(&isbn, api_key.as_deref()).await;
    result.existing_edition_id = existing_edition_id;
    Ok(result)
}

#[tauri::command]
pub fn list_books(database: State<'_, Database>) -> Result<Vec<BookSummary>> {
    let conn = database.lock()?;
    catalog::list_books(&conn)
}

#[tauri::command]
pub fn get_book(database: State<'_, Database>, edition_id: String) -> Result<BookDetail> {
    let conn = database.lock()?;
    catalog::get_book(&conn, &edition_id)
}

#[tauri::command]
pub fn delete_book(database: State<'_, Database>, edition_id: String) -> Result<()> {
    let mut conn = database.lock()?;
    catalog::delete_book(&mut conn, &edition_id)
}

#[tauri::command]
pub fn add_copy(
    database: State<'_, Database>,
    edition_id: String,
    copy: CopyInput,
) -> Result<String> {
    let copy = copy.validate().map_err(Error::Validation)?;
    let mut conn = database.lock()?;
    catalog::add_copy(&mut conn, &edition_id, &copy)
}

#[tauri::command]
pub fn update_copy(database: State<'_, Database>, copy_id: String, copy: CopyInput) -> Result<()> {
    let copy = copy.validate().map_err(Error::Validation)?;
    let mut conn = database.lock()?;
    catalog::update_copy(&mut conn, &copy_id, &copy)
}

#[tauri::command]
pub fn delete_copy(database: State<'_, Database>, copy_id: String) -> Result<()> {
    let mut conn = database.lock()?;
    catalog::delete_copy(&mut conn, &copy_id)
}

#[tauri::command]
pub fn book_history(
    database: State<'_, Database>,
    edition_id: String,
) -> Result<Vec<HistoryEntry>> {
    let conn = database.lock()?;
    catalog::book_history(&conn, &edition_id)
}

#[tauri::command]
pub fn get_settings(database: State<'_, Database>) -> Result<Settings> {
    let conn = database.lock()?;
    settings::load(&conn)
}

#[tauri::command]
pub fn save_settings(database: State<'_, Database>, settings: Settings) -> Result<()> {
    let conn = database.lock()?;
    settings::save(&conn, &settings)
}

#[tauri::command]
pub fn covers_dir(covers: State<'_, Covers>) -> String {
    covers.dir().display().to_string()
}
