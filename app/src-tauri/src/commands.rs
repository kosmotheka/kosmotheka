use std::path::Path;

use kosmotheka_domain::{CopyInput, EditionInput, FieldError, Isbn, NewBook, WishInput};
use serde::Serialize;
use tauri::State;
use url::Url;

use crate::backup::{self, BackupSummary, RestoreSummary};
use crate::catalog::{
    self, BookDetail, BookFilter, BookSummary, CatalogOptions, CreatedBook, HistoryEntry,
};
use crate::covers::Covers;
use crate::csv_io::{self, CsvImportSummary};
use crate::db::Database;
use crate::error::{Error, Result};
use crate::lan::service::{LanService, LanStatus, PairingInfo};
use crate::lan::{self, SyncReport};
use crate::libraries::{self, LibrarySummary};
use crate::marks;
use crate::metadata::{Lookup, LookupResult};
use crate::ops;
use crate::people::{self, People};
use crate::requests::{self, Request};
use crate::settings::{self, Settings};
use crate::wishes::{self, IsbnCheck, Wish};

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
    let library = ops::active_library(&conn)?;
    let created = catalog::create_book(&mut conn, &library, &book, cover.as_deref())?;
    Ok(Saved {
        value: created,
        cover_failed,
    })
}

/// What a write done in request mode became: applied, or kept for the
/// administrators to decide (D73).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Asked {
    pub requested: bool,
}

/// «Ask to add»: a reader asks for a book, with a note for whoever decides. The cover
/// is not downloaded: it would stay on this device, and whoever accepts can find it.
#[tauri::command]
pub fn request_book(
    database: State<'_, Database>,
    book: NewBook,
    note: Option<String>,
) -> Result<Asked> {
    let book = book.validate().map_err(Error::Validation)?;
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    let (_, requested) = ops::as_request(note.as_deref(), || {
        catalog::create_book(&mut conn, &library, &book, None)
    })?;
    Ok(Asked { requested })
}

/// «Ask to delete» a copy that someone else added.
#[tauri::command]
pub fn request_delete_copy(
    database: State<'_, Database>,
    copy_id: String,
    note: Option<String>,
) -> Result<Asked> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    let (_, requested) = ops::as_request(note.as_deref(), || {
        catalog::delete_copy(&mut conn, &library, &copy_id)
    })?;
    Ok(Asked { requested })
}

/// «Ask to delete» a whole book with rows that someone else added.
#[tauri::command]
pub fn request_delete_book(
    database: State<'_, Database>,
    edition_id: String,
    note: Option<String>,
) -> Result<Asked> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    let (_, requested) = ops::as_request(note.as_deref(), || {
        catalog::delete_book(&mut conn, &library, &edition_id)
    })?;
    Ok(Asked { requested })
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
    let library = ops::active_library(&conn)?;
    catalog::update_edition(&mut conn, &library, &edition_id, &edition, cover.as_deref())?;
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
            catalog::edition_with_isbn(&conn, &ops::active_library(&conn)?, isbn.as_str())?,
        )
    };
    let mut result = lookup.run(&isbn, api_key.as_deref()).await;
    result.existing_edition_id = existing_edition_id;
    Ok(result)
}

#[tauri::command]
pub fn list_books(
    database: State<'_, Database>,
    filter: Option<BookFilter>,
    everywhere: Option<bool>,
) -> Result<Vec<BookSummary>> {
    let conn = database.lock()?;
    let filter = filter.unwrap_or_default();
    if everywhere.unwrap_or(false) {
        return catalog::list_books_everywhere(&conn, &filter);
    }
    let library = ops::active_library(&conn)?;
    catalog::list_books(&conn, &library, &filter)
}

#[tauri::command]
pub fn catalog_options(database: State<'_, Database>) -> Result<CatalogOptions> {
    let conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    catalog::catalog_options(&conn, &library)
}

#[tauri::command]
pub fn get_book(database: State<'_, Database>, edition_id: String) -> Result<BookDetail> {
    let conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    catalog::get_book(&conn, &library, &edition_id)
}

#[tauri::command]
pub fn delete_book(database: State<'_, Database>, edition_id: String) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    catalog::delete_book(&mut conn, &library, &edition_id)
}

#[tauri::command]
pub fn add_copy(
    database: State<'_, Database>,
    edition_id: String,
    copy: CopyInput,
) -> Result<String> {
    let copy = copy.validate().map_err(Error::Validation)?;
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    catalog::add_copy(&mut conn, &library, &edition_id, &copy)
}

#[tauri::command]
pub fn update_copy(database: State<'_, Database>, copy_id: String, copy: CopyInput) -> Result<()> {
    let copy = copy.validate().map_err(Error::Validation)?;
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    catalog::update_copy(&mut conn, &library, &copy_id, &copy)
}

#[tauri::command]
pub fn delete_copy(database: State<'_, Database>, copy_id: String) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    catalog::delete_copy(&mut conn, &library, &copy_id)
}

#[tauri::command]
pub fn book_history(
    database: State<'_, Database>,
    edition_id: String,
) -> Result<Vec<HistoryEntry>> {
    let conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    catalog::book_history(&conn, &library, &edition_id)
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

// Paths come from the file dialogs the user just answered.

#[tauri::command]
pub fn export_backup(
    database: State<'_, Database>,
    covers: State<'_, Covers>,
    path: String,
    library: Option<String>,
) -> Result<BackupSummary> {
    let conn = database.lock()?;
    let library = match library {
        Some(library) => library,
        None => ops::active_library(&conn)?,
    };
    backup::export(&conn, &[library], &covers, Path::new(&path))
}

/// Every library on this device in one file (milestone 4.6).
#[tauri::command]
pub fn export_device_backup(
    database: State<'_, Database>,
    covers: State<'_, Covers>,
    path: String,
) -> Result<BackupSummary> {
    let conn = database.lock()?;
    backup::export(
        &conn,
        &backup::all_libraries(&conn)?,
        &covers,
        Path::new(&path),
    )
}

#[tauri::command]
pub fn restore_backup(
    database: State<'_, Database>,
    covers: State<'_, Covers>,
    path: String,
) -> Result<RestoreSummary> {
    let mut conn = database.lock()?;
    let active = ops::active_library(&conn)?;
    backup::restore(&mut conn, &active, &covers, Path::new(&path))
}

#[tauri::command]
pub fn export_csv(database: State<'_, Database>, path: String) -> Result<usize> {
    let conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    csv_io::export(&conn, &library, Path::new(&path))
}

#[tauri::command]
pub fn import_csv(database: State<'_, Database>, path: String) -> Result<CsvImportSummary> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    csv_io::import(&mut conn, &library, Path::new(&path))
}

// --- LAN sync ---------------------------------------------------------------

#[tauri::command]
pub async fn lan_status(lan: State<'_, LanService>) -> Result<LanStatus> {
    lan.status().await
}

#[tauri::command]
pub fn set_device_name(database: State<'_, Database>, name: String) -> Result<()> {
    lan::set_device_name(&*database.lock()?, &name)
}

#[tauri::command]
pub async fn open_pairing(
    lan: State<'_, LanService>,
    libraries: Vec<String>,
) -> Result<PairingInfo> {
    lan.open_pairing(libraries).await
}

#[tauri::command]
pub fn close_pairing(lan: State<'_, LanService>) -> Result<()> {
    lan.close_pairing()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Paired {
    pub device_id: String,
    pub name: String,
}

#[tauri::command]
pub async fn pair_with(
    database: State<'_, Database>,
    code: String,
    libraries: Vec<String>,
) -> Result<Paired> {
    let server = lan::client::pair(&database, &code, &libraries).await?;
    Ok(Paired {
        device_id: server.device_id,
        name: server.name,
    })
}

#[tauri::command]
pub async fn sync_with(database: State<'_, Database>, device_id: String) -> Result<SyncReport> {
    lan::client::sync(&database, &device_id).await
}

#[tauri::command]
pub fn forget_device(database: State<'_, Database>, device_id: String) -> Result<()> {
    lan::forget_device(&*database.lock()?, &device_id)
}

// --- Wishlist and "do I have it?" ---------------------------------------------

#[tauri::command]
pub fn add_wish(database: State<'_, Database>, wish: WishInput) -> Result<String> {
    let wish = wish.validate().map_err(Error::Validation)?;
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    wishes::add_wish(&mut conn, &library, &wish)
}

#[tauri::command]
pub fn list_wishes(database: State<'_, Database>) -> Result<Vec<Wish>> {
    let conn = database.lock()?;
    wishes::list_wishes(&conn, &ops::active_library(&conn)?)
}

#[tauri::command]
pub fn delete_wish(database: State<'_, Database>, id: String) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    wishes::delete_wish(&mut conn, &library, &id)
}

#[tauri::command]
pub fn check_isbn(database: State<'_, Database>, isbn: String) -> Result<IsbnCheck> {
    let conn = database.lock()?;
    wishes::check_isbn(&conn, &isbn)
}

// --- Libraries ------------------------------------------------------------------

#[tauri::command]
pub fn list_libraries(database: State<'_, Database>) -> Result<Vec<LibrarySummary>> {
    libraries::list_libraries(&*database.lock()?)
}

#[tauri::command]
pub fn create_library(database: State<'_, Database>, name: String) -> Result<String> {
    libraries::create_library(&mut *database.lock()?, &name)
}

#[tauri::command]
pub fn rename_library(database: State<'_, Database>, id: String, name: String) -> Result<()> {
    libraries::rename_library(&mut *database.lock()?, &id, &name)
}

#[tauri::command]
pub fn set_active_library(database: State<'_, Database>, id: String) -> Result<()> {
    libraries::set_active_library(&*database.lock()?, &id)
}

#[tauri::command]
pub fn delete_library(database: State<'_, Database>, id: String) -> Result<String> {
    libraries::delete_library(&mut *database.lock()?, &id)
}

#[tauri::command]
pub fn leave_library(database: State<'_, Database>, id: String) -> Result<String> {
    libraries::leave_library(&mut *database.lock()?, &id)
}

#[tauri::command]
pub fn sharing_overview(database: State<'_, Database>) -> Result<Vec<lan::DeviceSharing>> {
    lan::sharing_overview(&*database.lock()?)
}

/// Shares a library with a paired device, or withdraws its access (D71).
#[tauri::command]
pub fn set_sharing(
    database: State<'_, Database>,
    library: String,
    device_id: String,
    shared: bool,
) -> Result<()> {
    let mut conn = database.lock()?;
    if shared {
        libraries::grant_access(&mut conn, &library, &device_id)
    } else {
        libraries::revoke_access(&mut conn, &library, &device_id)
    }
}

// --- People and roles (D55, D73, D58) --------------------------------------------

#[tauri::command]
pub fn list_people(database: State<'_, Database>) -> Result<People> {
    let conn = database.lock()?;
    people::list_people(&conn, &ops::active_library(&conn)?)
}

/// Turns roles on, saying in the same step whose each device already sharing the
/// library is (D73).
#[tauri::command]
pub fn start_roles(
    database: State<'_, Database>,
    name: String,
    owners: Vec<people::DeviceOwner>,
) -> Result<String> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    people::start_roles(&mut conn, &library, &name, &owners)
}

/// People known in other libraries of this device, to reuse in the active one.
#[tauri::command]
pub fn known_people(database: State<'_, Database>) -> Result<Vec<people::KnownPerson>> {
    let conn = database.lock()?;
    people::known_elsewhere(&conn, &ops::active_library(&conn)?)
}

/// Whose a newly paired device is, with the role its person gets in each library
/// where they have none yet (a role is chosen, never inherited).
// --- Marks (D68) -----------------------------------------------------------------

#[tauri::command]
pub fn work_marks(database: State<'_, Database>, work_id: String) -> Result<marks::WorkMarks> {
    let conn = database.lock()?;
    marks::work_marks(&conn, &ops::active_library(&conn)?, &work_id)
}

#[tauri::command]
pub fn set_mark(database: State<'_, Database>, work_id: String, mark: marks::Mark) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    marks::set_mark(&mut conn, &library, &work_id, &mark)
}

#[tauri::command]
pub fn claim_candidates(database: State<'_, Database>) -> Result<Vec<marks::Candidate>> {
    let conn = database.lock()?;
    marks::claim_candidates(&conn, &ops::active_library(&conn)?)
}

#[tauri::command]
pub fn claim_person(database: State<'_, Database>, person: people::PersonPick) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    marks::claim_person(&mut conn, &library, &person)
}

#[tauri::command]
pub fn pairing_owners(
    database: State<'_, Database>,
    device: String,
) -> Result<people::PairingOwners> {
    let conn = database.lock()?;
    people::pairing_owners(&conn, &device)
}

#[tauri::command]
pub fn give_access(
    database: State<'_, Database>,
    device: String,
    person: people::PersonPick,
    roles: std::collections::BTreeMap<String, String>,
    can_share: bool,
) -> Result<()> {
    people::give_access(&mut *database.lock()?, &device, &person, &roles, can_share)
}

#[tauri::command]
pub fn set_role(
    database: State<'_, Database>,
    member: String,
    role: Option<String>,
    can_share: bool,
) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    people::set_role(&mut conn, &library, &member, role.as_deref(), can_share)
}

#[tauri::command]
pub fn assign_device(
    database: State<'_, Database>,
    device: String,
    member: Option<String>,
) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    people::assign_device(&mut conn, &library, &device, member.as_deref())
}

/// A lost device: out of every library, wherever it is (D58).
#[tauri::command]
pub fn remove_device(database: State<'_, Database>, device: String) -> Result<usize> {
    people::remove_device(&mut *database.lock()?, &device)
}

#[tauri::command]
pub fn duplicate_library(
    database: State<'_, Database>,
    id: String,
    name: String,
) -> Result<String> {
    backup::duplicate(&mut *database.lock()?, &id, &name)
}

// --- Requests (D73) -------------------------------------------------------------

#[tauri::command]
pub fn list_requests(database: State<'_, Database>) -> Result<Vec<Request>> {
    let conn = database.lock()?;
    requests::list_requests(&conn, &ops::active_library(&conn)?)
}

#[tauri::command]
pub fn accept_request(database: State<'_, Database>, id: String) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    requests::accept(&mut conn, &library, &id)
}

#[tauri::command]
pub fn reject_request(database: State<'_, Database>, id: String) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    requests::reject(&mut conn, &library, &id)
}

/// What a book request asked for, as the add form fills it in.
#[tauri::command]
pub fn requested_book(database: State<'_, Database>, id: String) -> Result<requests::BookDraft> {
    let conn = database.lock()?;
    requests::requested_book(&conn, &ops::active_library(&conn)?, &id)
}

/// «Edit and accept»: adds the book as corrected, in the operation that accepts
/// the request (D73).
#[tauri::command]
pub async fn accept_request_edited(
    database: State<'_, Database>,
    covers: State<'_, Covers>,
    lookup: State<'_, Lookup>,
    id: String,
    book: NewBook,
) -> Result<Saved<CreatedBook>> {
    let book = book.validate().map_err(Error::Validation)?;
    let (cover, cover_failed) =
        download_cover(&covers, &lookup, book.edition.cover_url.as_ref()).await;
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    let created =
        catalog::create_book_accepting(&mut conn, &library, &book, cover.as_deref(), Some(&id))?;
    Ok(Saved {
        value: created,
        cover_failed,
    })
}

#[tauri::command]
pub fn withdraw_request(database: State<'_, Database>, id: String) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    requests::withdraw(&mut conn, &library, &id)
}

/// This device's changes that became requests on recalculating the active library.
#[tauri::command]
pub fn request_notices(database: State<'_, Database>) -> Result<Vec<Request>> {
    let conn = database.lock()?;
    requests::notices(&conn, &ops::active_library(&conn)?)
}

#[tauri::command]
pub fn dismiss_request_notices(database: State<'_, Database>) -> Result<()> {
    let conn = database.lock()?;
    requests::dismiss_notices(&conn, &ops::active_library(&conn)?)
}

#[tauri::command]
pub fn library_devices(database: State<'_, Database>, id: String) -> Result<Vec<lan::KnownDevice>> {
    lan::devices_sharing(&*database.lock()?, &id)
}

/// Moves a book from the active library to `to`. Returns its new edition id.
#[tauri::command]
pub fn move_book(database: State<'_, Database>, edition_id: String, to: String) -> Result<String> {
    let mut conn = database.lock()?;
    let from = ops::active_library(&conn)?;
    catalog::move_book(&mut conn, &from, &edition_id, &to)
}

/// The UI shows the camera scanner only where the plugin exists (D35).
#[tauri::command]
pub fn is_mobile() -> bool {
    cfg!(mobile)
}
