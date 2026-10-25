use kosmotheka_domain::{CopyInput, EditionInput, FieldError, Isbn, Kind, NewBook};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_fs::{FilePath, FsExt, OpenOptions};
use url::Url;

use crate::account::{
    self, AccountDetails, AccountDevice, AccountStatus, AccountUser, ResetDeps, ServerInfo,
    TwoFactorSetup, client::ActionProof,
};
use crate::backup::{self, BackupSummary, RestoreSummary};
use crate::catalog::{
    self, BookDetail, BookFilter, BookSummary, CatalogOptions, CreatedBook, HistoryEntry,
    IsbnCheck, TagSummary,
};
use crate::cloud::events::{Connection, EventsService};
use crate::cloud::invitations::{
    self, Accepted, InvitationItem, InviteRequest, InviteResult, InviteStatus, OpenedInvitation,
    PendingInvitation,
};
use crate::cloud::{self, LibraryCloudStatus, LibraryReport, client::HttpCloud};
use crate::covers::Covers;
use crate::csv_io::{self, CsvImportSummary};
use crate::db::Database;
use crate::error::{Error, Result};
use crate::lan::service::{LanService, LanStatus, PairingInfo};
use crate::lan::{self, SyncReport};
use crate::libraries::{self, LibrarySummary};
use crate::licenses;
use crate::links::{self, Arrivals};
use crate::marks;
use crate::metadata::bgg::BGG;
use crate::metadata::gameupc::{self, GAMEUPC};
use crate::metadata::via_server::{COMIC_VINE, GOOGLE_BOOKS, RAWG, TMDB, choose_access};
use crate::metadata::{Candidate, CodeResult, Lookup, LookupResult, SearchResult, Source};
use crate::ops;
use crate::people::{self, People};
use crate::requests::{self, Request};
use crate::series::{self, SeriesDetail, SeriesSummary};
use crate::settings::{self, Settings};
use crate::whats_new::{self, WhatsNew};

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
    let own = book.edition.own_cover.clone();
    let book = book.validate().map_err(Error::Validation)?;
    // Download before locking: the database lock must never be held while waiting on the network.
    let (cover, cover_failed) = cover_for(
        &covers,
        &lookup,
        own.as_deref(),
        book.edition.cover_url.as_ref(),
    )
    .await?;
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

/// Catalogs a work with no copy yet (D61): the next volume of a saga, the
/// expansion we lack. Editors and admins do this directly.
#[tauri::command]
pub async fn add_missing(
    database: State<'_, Database>,
    covers: State<'_, Covers>,
    lookup: State<'_, Lookup>,
    edition: EditionInput,
) -> Result<Saved<CreatedBook>> {
    let own = edition.own_cover.clone();
    let edition = edition.validate().map_err(Error::Validation)?;
    // Download before locking: the database lock must never be held while waiting on the network.
    let (cover, cover_failed) =
        cover_for(&covers, &lookup, own.as_deref(), edition.cover_url.as_ref()).await?;
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    let created = catalog::create_missing(&mut conn, &library, &edition, cover.as_deref())?;
    Ok(Saved {
        value: created,
        cover_failed,
    })
}

/// «Ask to catalog»: a reader asks for a missing work, with a note for whoever
/// decides. The cover is not downloaded, like `request_book`.
#[tauri::command]
pub fn request_missing(
    database: State<'_, Database>,
    edition: EditionInput,
    note: Option<String>,
) -> Result<Asked> {
    let edition = edition.validate().map_err(Error::Validation)?;
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    let (_, requested) = ops::as_request(note.as_deref(), || {
        catalog::create_missing(&mut conn, &library, &edition, None)
    })?;
    Ok(Asked { requested })
}

/// «Ask to add a copy»: a reader asks to add a copy of a work they are missing,
/// with a note for whoever decides.
#[tauri::command]
pub fn request_add_copy(
    database: State<'_, Database>,
    edition_id: String,
    copy: CopyInput,
    note: Option<String>,
) -> Result<Asked> {
    let copy = copy.validate().map_err(Error::Validation)?;
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    let (_, requested) = ops::as_request(note.as_deref(), || {
        catalog::add_copy(&mut conn, &library, &edition_id, &copy)
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
    let own = edition.own_cover.clone();
    let edition = edition.validate().map_err(Error::Validation)?;
    let (cover, cover_failed) =
        cover_for(&covers, &lookup, own.as_deref(), edition.cover_url.as_ref()).await?;
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    catalog::update_edition(&mut conn, &library, &edition_id, &edition, cover.as_deref())?;
    Ok(Saved {
        value: (),
        cover_failed,
    })
}

/// Stores a cover of one's own, sent by the interface in base64 (on Android, Tauri's calls carry JSON): a photo or an
/// image of the device, reduced like any other (D82). Returns the file name to save the edition with.
#[tauri::command]
pub fn store_own_cover(covers: State<'_, Covers>, photo: String) -> Result<String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(photo)
        .map_err(|_| Error::CoverRejected("not an image"))?;
    covers.store_own(&bytes)
}

/// The cover an edition is saved with: one of its own already stored here (`own_cover`), or the remote one,
/// downloaded. Also whether a download was attempted and failed. An own cover that is not one of this device's covers
/// is refused, not ignored.
async fn cover_for(
    covers: &Covers,
    lookup: &Lookup,
    own: Option<&str>,
    url: Option<&Url>,
) -> Result<(Option<String>, bool)> {
    match own {
        Some(name) if covers.has(name) => Ok((Some(name.to_owned()), false)),
        Some(_) => Err(Error::CoverRejected("not one of this device's covers")),
        None => Ok(download_cover(covers, lookup, url).await),
    }
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

/// The account's server, only when it could be asked about a book (7.8, D83): this device has a session, Google
/// Books is on, there is no key of their own (which has preference) and the person has not switched it off. Nothing
/// else ever reaches the server.
fn books_cloud(database: &Database, settings: &Settings) -> Option<HttpCloud> {
    let own_key = settings
        .api_keys
        .get("google_books")
        .is_some_and(|key| !key.trim().is_empty());
    let allowed = settings.google_books_via_server
        && !own_key
        && !settings.disabled_sources.contains(Source::GoogleBooks.id());
    allowed.then(|| HttpCloud::connect(database).ok()).flatten()
}

/// Which catalogues the server of this device's session looks up for its accounts (`google_books`, `tmdb`, `rawg`, `comic_vine`, `bgg`, `gameupc`), so
/// Providers shows what is worth showing (7.8). Any failure, and no session, is an empty list.
#[tauri::command]
pub async fn cloud_metadata_offers(database: State<'_, Database>) -> Result<Vec<String>> {
    let Ok(cloud) = HttpCloud::connect(&database) else {
        return Ok(Vec::new());
    };
    // One request: the server lists them all. Only those this app knows are kept, in its order.
    let listed = cloud.offered_metadata().await.unwrap_or_default();
    Ok([GOOGLE_BOOKS, TMDB, RAWG, COMIC_VINE, BGG, GAMEUPC]
        .into_iter()
        .filter(|provider| listed.iter().any(|offered| offered == provider))
        .map(str::to_owned)
        .collect())
}

/// The search services the server of the session asks for its accounts, and those this device has not been told
/// about yet (D91): the screen that tells them shows when there is one. Empty without a session.
#[tauri::command]
pub async fn server_services(database: State<'_, Database>) -> Result<settings::ServerServices> {
    let offered = cloud_metadata_offers(database.clone()).await?;
    let conn = database.lock()?;
    let server = crate::account::server(&conn)?;
    settings::server_services(&conn, &server, offered)
}

#[tauri::command]
pub fn mark_server_services_seen(
    database: State<'_, Database>,
    services: Vec<String>,
) -> Result<()> {
    let conn = database.lock()?;
    let server = crate::account::server(&conn)?;
    settings::mark_server_services_seen(&conn, &server, &services)
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
    let (settings, existing_edition_id) = {
        let conn = database.lock()?;
        (
            settings::load(&conn)?,
            catalog::edition_with_isbn(&conn, &ops::active_library(&conn)?, isbn.as_str())?,
        )
    };
    let api_key = settings.api_keys.get("google_books").map(String::as_str);
    let cloud = books_cloud(&database, &settings);
    let mut result = lookup
        .run(
            &isbn,
            choose_access(api_key, settings.google_books_via_server, cloud.as_ref()),
            &settings.disabled_sources,
        )
        .await;
    result.existing_edition_id = existing_edition_id;
    Ok(result)
}

#[tauri::command]
pub async fn search_works(
    database: State<'_, Database>,
    lookup: State<'_, Lookup>,
    kind: Kind,
    query: String,
    language: Option<String>,
) -> Result<SearchResult> {
    search(&database, &lookup, kind, &query, language.as_deref()).await
}

/// The shelf photo: one text read on a shelf, searched as if typed, and the first result whose title is what was
/// read (`shelf::first_match`), or none. The catalogues' warnings are left out: a shelf is many searches, and one
/// that fails is one book less, not a message each time.
#[tauri::command]
pub async fn shelf_match(
    database: State<'_, Database>,
    lookup: State<'_, Lookup>,
    kind: Kind,
    read: String,
    language: Option<String>,
) -> Result<Option<Candidate>> {
    let mut found = search(&database, &lookup, kind, &read, language.as_deref()).await?;
    let titles = found.candidates.iter().map(|c| c.title.as_str());
    let matched = kosmotheka_domain::shelf::first_match(&read, titles);
    Ok(matched.map(|i| found.candidates.swap_remove(i)))
}

async fn search(
    database: &Database,
    lookup: &Lookup,
    kind: Kind,
    query: &str,
    language: Option<&str>,
) -> Result<SearchResult> {
    let settings = {
        let conn = database.lock()?;
        settings::load(&conn)?
    };
    let disabled_sources = settings.disabled_sources.clone();
    // TMDB (films, series), RAWG (video games) and Comic Vine (comics) with the person's own key, or through the
    // account's server; BoardGameGeek (board games) and RPGGeek (role-playing books, among the books) only through
    // the server, as they have no key of their own. The server is only looked for when there is no key of their own.
    let source = match kind {
        Kind::Film | Kind::TvSeries => Some(Source::Tmdb),
        Kind::Videogame => Some(Source::Rawg),
        Kind::Comic => Some(Source::ComicVine),
        Kind::Boardgame => Some(Source::BoardGameGeek),
        Kind::Book => Some(Source::RpgGeek),
        _ => None,
    };
    let own_key = source
        .and_then(|source| settings.api_keys.get(source.id()))
        .cloned();
    let cloud = source
        .filter(|source| !settings.disabled_sources.contains(source.id()))
        .filter(|_| own_key.as_deref().is_none_or(|key| key.trim().is_empty()))
        .and_then(|_| HttpCloud::connect(database).ok());
    let language = if language == Some("es") { "es" } else { "en" };
    let mut result = lookup
        .search(
            kind,
            query,
            &disabled_sources,
            own_key.as_deref(),
            cloud.as_ref(),
            language,
        )
        .await;
    // The block ends before this function returns and there is no further await,
    // so the lock is never held across one.
    let conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    for candidate in &mut result.candidates {
        candidate.existing_edition_id = catalog::edition_matching_candidate(
            &conn,
            &library,
            candidate.isbn.as_deref(),
            candidate.barcode.as_deref(),
        )?;
    }
    Ok(result)
}

#[tauri::command]
pub async fn lookup_code(
    database: State<'_, Database>,
    lookup: State<'_, Lookup>,
    code: String,
) -> Result<CodeResult> {
    let (settings, existing_edition_id) = {
        let conn = database.lock()?;
        let library = ops::active_library(&conn)?;
        let existing = match Isbn::parse(&code) {
            Ok(isbn) => catalog::edition_with_isbn(&conn, &library, isbn.as_str())?,
            Err(_) => catalog::edition_with_barcode(&conn, &library, &code)?,
        };
        (settings::load(&conn)?, existing)
    };
    let cloud = books_cloud(&database, &settings);
    // GameUPC only through the account's server, with no key of their own (D62).
    let gameupc_cloud = (!settings.disabled_sources.contains(Source::GameUpc.id()))
        .then(|| HttpCloud::connect(&database).ok())
        .flatten();
    let mut result = lookup
        .lookup_code(
            &code,
            choose_access(
                settings.api_keys.get("google_books").map(String::as_str),
                settings.google_books_via_server,
                cloud.as_ref(),
            ),
            gameupc_cloud.as_ref(),
            &settings.disabled_sources,
        )
        .await;
    result.existing_edition_id = existing_edition_id;
    Ok(result)
}

/// Sends GameUPC the game a person chose for a barcode it was not sure of, once the game is in the catalogue (D62).
/// Best effort, and silent: without a session, with GameUPC turned off or with a server that does not offer it,
/// nothing is sent, and a failure only means GameUPC learns nothing this time.
#[tauri::command]
pub async fn gameupc_choice(
    database: State<'_, Database>,
    code: String,
    external_id: String,
) -> Result<()> {
    let disabled = {
        let conn = database.lock()?;
        settings::load(&conn)?
            .disabled_sources
            .contains(Source::GameUpc.id())
    };
    let Ok(external_id) = external_id.parse::<u64>() else {
        return Ok(());
    };
    if disabled {
        return Ok(());
    }
    if let Ok(cloud) = HttpCloud::connect(&database) {
        gameupc::send_choice(&cloud, &code, external_id).await;
    }
    Ok(())
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
pub fn list_tags(database: State<'_, Database>) -> Result<Vec<TagSummary>> {
    let conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    catalog::list_tags(&conn, &library)
}

#[tauri::command]
pub fn rename_tag(database: State<'_, Database>, id: String, name: String) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    catalog::rename_tag(&mut conn, &library, &id, &name)
}

#[tauri::command]
pub fn delete_tag(database: State<'_, Database>, id: String) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    catalog::delete_tag(&mut conn, &library, &id)
}

#[tauri::command]
pub fn genres_of_kind(kind: String) -> Vec<String> {
    kosmotheka_domain::genres_of(&kind)
        .iter()
        .map(|genre| genre.to_string())
        .collect()
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

/// Opens a file the user just picked. On Android it is a `content://` URI that
/// only Android can open, so the fs plugin asks it for the file.
fn open_picked(app: &AppHandle, path: &str, write: bool) -> Result<std::fs::File> {
    let Ok(path) = path.parse::<FilePath>();
    let mut options = OpenOptions::new();
    if write {
        options.write(true).create(true).truncate(true);
    } else {
        options.read(true);
    }
    Ok(app.fs().open(path, options)?)
}

#[tauri::command]
pub fn export_backup(
    app: AppHandle,
    database: State<'_, Database>,
    covers: State<'_, Covers>,
    path: String,
    library: Option<String>,
    passphrase: Option<String>,
) -> Result<BackupSummary> {
    let conn = database.lock()?;
    let library = match library {
        Some(library) => library,
        None => ops::active_library(&conn)?,
    };
    let file = open_picked(&app, &path, true)?;
    backup::export(&conn, &[library], &covers, file, passphrase.as_deref())
}

/// Every library on this device in one file (milestone 4.6).
#[tauri::command]
pub fn export_device_backup(
    app: AppHandle,
    database: State<'_, Database>,
    covers: State<'_, Covers>,
    path: String,
    passphrase: Option<String>,
) -> Result<BackupSummary> {
    let conn = database.lock()?;
    let file = open_picked(&app, &path, true)?;
    backup::export(
        &conn,
        &backup::all_libraries(&conn)?,
        &covers,
        file,
        passphrase.as_deref(),
    )
}

#[tauri::command]
pub fn restore_backup(
    app: AppHandle,
    database: State<'_, Database>,
    covers: State<'_, Covers>,
    path: String,
    passphrase: Option<String>,
) -> Result<RestoreSummary> {
    let mut conn = database.lock()?;
    let active = ops::active_library(&conn)?;
    let file = open_picked(&app, &path, false)?;
    backup::restore(&mut conn, &active, &covers, file, passphrase.as_deref())
}

#[tauri::command]
pub fn export_csv(app: AppHandle, database: State<'_, Database>, path: String) -> Result<usize> {
    let conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    csv_io::export(&conn, &library, open_picked(&app, &path, true)?)
}

/// The list of third-party licences, built from what the app carries (no text comes from the UI).
#[tauri::command]
pub fn export_licenses(app: AppHandle, path: String) -> Result<()> {
    licenses::export(
        open_picked(&app, &path, true)?,
        &app.package_info().version.to_string(),
    )
}

#[tauri::command]
pub fn import_csv(
    app: AppHandle,
    database: State<'_, Database>,
    path: String,
) -> Result<CsvImportSummary> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    csv_io::import(&mut conn, &library, open_picked(&app, &path, false)?)
}

// --- LAN sync ---------------------------------------------------------------

#[tauri::command]
pub async fn lan_status(lan: State<'_, LanService>) -> Result<LanStatus> {
    lan.status().await
}

#[tauri::command]
pub fn set_device_name(database: State<'_, Database>, name: String) -> Result<()> {
    let mut conn = database.lock()?;
    lan::set_device_name(&mut conn, &name)
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

/// The devices nearby with a pairing window open, to pair by number (D88).
#[tauri::command]
pub async fn find_pairing_devices(
    database: State<'_, Database>,
) -> Result<Vec<lan::client::NearbyDevice>> {
    let own = ops::device_id(&*database.lock()?)?;
    Ok(lan::client::find_pairing(&own).await)
}

/// Pairs with the device at `address` with the six digits it shows (D88).
#[tauri::command]
pub async fn pair_with_pin(
    database: State<'_, Database>,
    address: String,
    pin: String,
    libraries: Vec<String>,
) -> Result<Paired> {
    let server = lan::client::pair_with_pin(&database, &address, pin.trim(), &libraries).await?;
    Ok(Paired {
        device_id: server.device_id,
        name: server.name,
    })
}

/// After the operations, the covers they point to are swapped in the background (D82): the library shows at once
/// and its covers follow. `covers-arrived` tells the UI to show the ones that came.
#[tauri::command]
pub async fn sync_with(
    app: AppHandle,
    database: State<'_, Database>,
    covers: State<'_, Covers>,
    device_id: String,
) -> Result<SyncReport> {
    let report = lan::client::sync(&database, &device_id).await?;
    let (database, covers) = (database.inner().clone(), covers.inner().clone());
    tauri::async_runtime::spawn(async move {
        if let Ok(swapped) = lan::client::exchange_covers(&database, &covers, &device_id).await
            && swapped.received > 0
        {
            let _ = app.emit("covers-arrived", ());
        }
    });
    Ok(report)
}

#[tauri::command]
pub fn forget_device(database: State<'_, Database>, device_id: String) -> Result<()> {
    lan::forget_device(&*database.lock()?, &device_id)
}

// --- "Do I have it?" -------------------------------------------------------------

#[tauri::command]
pub fn check_isbn(database: State<'_, Database>, isbn: String) -> Result<IsbnCheck> {
    let conn = database.lock()?;
    catalog::check_isbn(&conn, &isbn)
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

/// Removes the library from this device. Without another paired device having it, the cloud has to say, now,
/// that it still has it for this account: with no session, no connection or no such library, nothing changes
/// (`OnlyOnThisDevice`).
#[tauri::command]
pub async fn leave_library(database: State<'_, Database>, id: String) -> Result<String> {
    match HttpCloud::connect(&database) {
        Ok(api) => cloud::leave(&database, &api, &id).await,
        Err(_) => libraries::leave_library(&mut *database.lock()?, &id),
    }
}

/// Whether the cloud of the signed-in account offers this library right now: what the dialog asks before it
/// promises that the library stays in the account. Any failure is a no.
#[tauri::command]
pub async fn library_in_cloud(database: State<'_, Database>, id: String) -> Result<bool> {
    Ok(match HttpCloud::connect(&database) {
        Ok(api) => cloud::offers(&database, &api, &id).await,
        Err(_) => false,
    })
}

/// The libraries this device removed by its own choice and that the cloud may hand over again. Local, no network.
#[tauri::command]
pub fn left_libraries(database: State<'_, Database>) -> Result<Vec<libraries::LeftLibrary>> {
    libraries::left_libraries(&*database.lock()?)
}

/// «Bring it down again»: one of `left_libraries`, when the person asks. Nothing else can come back this way.
#[tauri::command]
pub async fn redownload_library(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    id: String,
) -> Result<LibraryReport> {
    let result = match HttpCloud::connect(&database) {
        Ok(api) => cloud::redownload(&database, &api, &id).await,
        Err(error) => Err(error),
    };
    let result = account::react_to_account_error(&database, reset.context(), result).await;
    emit_if_reset(&app, &result);
    result
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

/// People known in other libraries of this device, to reuse in the active one.
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
    libraries: Vec<String>,
) -> Result<()> {
    people::give_access(
        &mut *database.lock()?,
        &device,
        &person,
        &roles,
        can_share,
        &libraries,
    )
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

/// Sends a person away from the active library (D98): their devices lose it and their account stops
/// getting it. They stay in the catalog.
#[tauri::command]
pub fn withdraw_person(database: State<'_, Database>, member: String) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    people::withdraw_person(&mut conn, &library, &member)
}

/// «Leave the library» for good (D102): this device's person withdraws her own access in the active library.
/// The operation has to reach another device before the library goes from this one (see `people::leave_for_good`).
#[tauri::command]
pub fn leave_library_for_good(database: State<'_, Database>) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    people::leave_for_good(&mut conn, &library)
}

/// Takes the mark off a person sent away (D98). It gives nothing back by itself.
#[tauri::command]
pub fn restore_person(database: State<'_, Database>, member: String) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    people::restore_person(&mut conn, &library, &member)
}

/// A lost device: out of every library, wherever it is (D58).
#[tauri::command]
pub fn remove_device(database: State<'_, Database>, device: String) -> Result<usize> {
    people::remove_device(&mut *database.lock()?, &device)
}

/// What reinstalling left behind: this device's earlier installs, out of every library (D81).
#[tauri::command]
pub fn remove_earlier_installs(database: State<'_, Database>, library: String) -> Result<usize> {
    people::remove_earlier_installs(&mut *database.lock()?, &library)
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
    let own = book.edition.own_cover.clone();
    let book = book.validate().map_err(Error::Validation)?;
    let (cover, cover_failed) = cover_for(
        &covers,
        &lookup,
        own.as_deref(),
        book.edition.cover_url.as_ref(),
    )
    .await?;
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

/// The ISBNs printed in a photo, for a book whose barcode is a shop's label: the photo is read on the device with
/// the text recognition plugin, and only what the check digit confirms comes back (`Isbn::find_in_text`). The photo
/// comes in base64: on Android, Tauri's calls carry JSON, not raw bytes.
#[tauri::command]
pub async fn isbns_in_photo(app: tauri::AppHandle, photo: String) -> Result<Vec<String>> {
    use base64::Engine;
    let photo = base64::engine::general_purpose::STANDARD
        .decode(photo)
        .map_err(|_| Error::TextRecognition("the photo is not base64".into()))?;
    crate::photo_text::isbns_in_photo(&app, &photo).await
}

/// The shelf photo: the lines read on its spines that are worth searching for (`shelf::queries`), read on the device
/// like the printed ISBN; the photo comes in base64 for the same reason.
#[tauri::command]
pub async fn shelf_queries(app: tauri::AppHandle, photo: String) -> Result<Vec<String>> {
    use base64::Engine;
    let photo = base64::engine::general_purpose::STANDARD
        .decode(photo)
        .map_err(|_| Error::TextRecognition("the photo is not base64".into()))?;
    crate::photo_text::shelf_queries(&app, &photo).await
}

/// The UI shows the camera scanner only where the plugin exists (D35).
#[tauri::command]
pub fn is_mobile() -> bool {
    cfg!(mobile)
}

/// Whether the fingerprint/face lock is on (D78). Per-device, never synced.
#[tauri::command]
pub fn app_lock_enabled(database: State<'_, Database>) -> Result<bool> {
    let conn = database.lock()?;
    settings::app_lock(&conn)
}

#[tauri::command]
pub fn set_app_lock(database: State<'_, Database>, enabled: bool) -> Result<()> {
    let conn = database.lock()?;
    settings::set_app_lock(&conn, enabled)
}

/// The name this device's person goes by (D97); `None` until the welcome has it.
#[tauri::command]
pub fn person_name(database: State<'_, Database>) -> Result<Option<String>> {
    let conn = database.lock()?;
    settings::person_name(&conn)
}

/// The welcome's first question: the name, and with it «Mi biblioteca» founded with that person as its
/// administrator.
#[tauri::command]
pub fn begin_with_name(database: State<'_, Database>, name: String) -> Result<()> {
    let mut conn = database.lock()?;
    libraries::begin(&mut conn, &name)
}

/// Changes the name the libraries created from now on are founded with. The ones that exist keep theirs.
#[tauri::command]
pub fn set_person_name(database: State<'_, Database>, name: String) -> Result<String> {
    let conn = database.lock()?;
    settings::set_person_name(&conn, &name)
}

#[tauri::command]
pub fn welcome_seen(database: State<'_, Database>) -> Result<u32> {
    let conn = database.lock()?;
    settings::welcome_seen(&conn)
}

/// What is new since the last version this device saw, to show once after an update.
#[tauri::command]
pub fn whats_new_pending(database: State<'_, Database>) -> Result<WhatsNew> {
    let conn = database.lock()?;
    whats_new::pending(&conn, env!("CARGO_PKG_VERSION"))
}

/// The latest versions' news, to read again from «About».
#[tauri::command]
pub fn whats_new_recent(database: State<'_, Database>) -> Result<WhatsNew> {
    let conn = database.lock()?;
    whats_new::recent(&conn, env!("CARGO_PKG_VERSION"))
}

#[tauri::command]
pub fn set_whats_new_switch(database: State<'_, Database>, key: String, on: bool) -> Result<()> {
    let conn = database.lock()?;
    whats_new::set_switch(&conn, &key, on)
}

#[tauri::command]
pub fn set_whats_new_seen(database: State<'_, Database>) -> Result<()> {
    let conn = database.lock()?;
    settings::set_whats_new_seen(&conn, env!("CARGO_PKG_VERSION"))
}

#[tauri::command]
pub fn set_welcome_seen(database: State<'_, Database>, version: u32) -> Result<()> {
    let conn = database.lock()?;
    settings::set_welcome_seen(&conn, version)
}

/// Minutes the app may stay in the background before the lock comes back.
#[tauri::command]
pub fn app_lock_after(database: State<'_, Database>) -> Result<u32> {
    let conn = database.lock()?;
    settings::app_lock_after(&conn)
}

#[tauri::command]
pub fn set_app_lock_after(database: State<'_, Database>, minutes: u32) -> Result<()> {
    let conn = database.lock()?;
    settings::set_app_lock_after(&conn, minutes)
}

#[tauri::command]
pub fn list_series(database: State<'_, Database>) -> Result<Vec<SeriesSummary>> {
    let conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    series::list_series(&conn, &library)
}

#[tauri::command]
pub fn get_series(database: State<'_, Database>, id: String) -> Result<SeriesDetail> {
    let conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    series::get_series(&conn, &library, &id)
}

#[tauri::command]
pub fn create_series(database: State<'_, Database>, name: String, ordered: bool) -> Result<String> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    series::create_series(&mut conn, &library, &name, ordered)
}

#[tauri::command]
pub fn update_series(
    database: State<'_, Database>,
    id: String,
    name: String,
    ordered: bool,
) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    series::update_series(&mut conn, &library, &id, &name, ordered)
}

#[tauri::command]
pub fn delete_series(database: State<'_, Database>, id: String) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    series::delete_series(&mut conn, &library, &id)
}

#[tauri::command]
pub fn add_to_series(
    database: State<'_, Database>,
    series_id: String,
    work_id: String,
    position: Option<i64>,
) -> Result<String> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    series::add_to_series(&mut conn, &library, &series_id, &work_id, position)
}

#[tauri::command]
pub fn remove_from_series(
    database: State<'_, Database>,
    series_id: String,
    work_id: String,
) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    series::remove_from_series(&mut conn, &library, &series_id, &work_id)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkPosition {
    pub work_id: String,
    pub position: Option<i64>,
}

#[tauri::command]
pub fn reorder_series(
    database: State<'_, Database>,
    series_id: String,
    positions: Vec<WorkPosition>,
) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    let positions: Vec<(String, Option<i64>)> = positions
        .into_iter()
        .map(|p| (p.work_id, p.position))
        .collect();
    series::reorder(&mut conn, &library, &series_id, &positions)
}

#[tauri::command]
pub fn add_requirement(
    database: State<'_, Database>,
    work_id: String,
    required_work_id: String,
) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    series::add_requirement(&mut conn, &library, &work_id, &required_work_id)
}

#[tauri::command]
pub fn remove_requirement(
    database: State<'_, Database>,
    work_id: String,
    required_work_id: String,
) -> Result<()> {
    let mut conn = database.lock()?;
    let library = ops::active_library(&conn)?;
    series::remove_requirement(&mut conn, &library, &work_id, &required_work_id)
}

// --- Account (D77, D81, hito 7.1) ------------------------------------------

/// Emits `device-reset` (no payload) so the UI reloads, once the reset has
/// actually happened: either this call always resets (`give_device_away`), or it
/// resets only when the server answered `device_removed` (every other call here).
pub fn emit_if_reset<T>(app: &AppHandle, result: &Result<T>) {
    if let Err(Error::Account { code, .. }) = result {
        // The session is gone in all three: nothing left to listen to (D75).
        if matches!(
            code.as_str(),
            "device_removed" | "signed_out_password_changed" | "invalid_token"
        ) {
            app.state::<EventsService>().stop();
        }
        if code == "device_removed" {
            let _ = app.emit("device-reset", ());
        }
    }
}

#[tauri::command]
pub fn account_status(database: State<'_, Database>) -> Result<AccountStatus> {
    account::account_status(&database)
}

#[tauri::command]
pub fn dismiss_account_notice(database: State<'_, Database>) -> Result<()> {
    account::dismiss_account_notice(&database)
}

#[tauri::command]
pub fn set_account_server(database: State<'_, Database>, url: Option<String>) -> Result<()> {
    account::set_server(&database, url)
}

#[tauri::command]
pub async fn account_server_info(database: State<'_, Database>) -> Result<ServerInfo> {
    account::server_info(&database).await
}

#[tauri::command]
pub async fn register_account(
    database: State<'_, Database>,
    email: String,
    password: String,
    name: String,
    locale: String,
) -> Result<()> {
    account::register(&database, &email, &password, &name, &locale).await
}

#[tauri::command]
pub async fn confirm_account_email(
    database: State<'_, Database>,
    email: String,
    code: String,
    password: String,
) -> Result<()> {
    account::confirm_email(&database, &email, &code, &password).await
}

#[tauri::command]
pub async fn resend_account_code(database: State<'_, Database>, email: String) -> Result<()> {
    account::resend_code(&database, &email).await
}

#[tauri::command]
pub async fn sign_in(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    email: String,
    password: String,
    totp_code: Option<String>,
) -> Result<AccountUser> {
    let result = account::sign_in(&database, reset.context(), &email, &password, totp_code).await;
    emit_if_reset(&app, &result);
    if result.is_ok() {
        // Also restarts it for another account or server.
        app.state::<EventsService>().start();
    }
    result
}

#[tauri::command]
pub async fn sign_out(app: AppHandle, database: State<'_, Database>) -> Result<()> {
    let result = account::sign_out(&database).await;
    app.state::<EventsService>().stop();
    result
}

#[tauri::command]
pub async fn forgot_password(database: State<'_, Database>, email: String) -> Result<()> {
    account::forgot_password(&database, &email).await
}

#[tauri::command]
pub async fn reset_password(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    email: String,
    code: String,
    password: String,
    totp_code: Option<String>,
) -> Result<()> {
    let result = account::reset_password(
        &database,
        reset.context(),
        &email,
        &code,
        &password,
        totp_code.as_deref(),
    )
    .await;
    emit_if_reset(&app, &result);
    result
}

#[tauri::command]
pub async fn get_account(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
) -> Result<AccountDetails> {
    let result = account::get_account(&database, reset.context()).await;
    emit_if_reset(&app, &result);
    result
}

#[tauri::command]
pub async fn update_account(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    name: Option<String>,
    locale: Option<String>,
) -> Result<AccountDetails> {
    let result = account::update_account(&database, reset.context(), name, locale).await;
    emit_if_reset(&app, &result);
    result
}

#[tauri::command]
pub async fn change_password(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    current_password: String,
    new_password: String,
    totp_code: Option<String>,
) -> Result<()> {
    let result = account::change_password(
        &database,
        reset.context(),
        &current_password,
        &new_password,
        totp_code.as_deref(),
    )
    .await;
    emit_if_reset(&app, &result);
    result
}

#[tauri::command]
pub async fn delete_account(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    proof: ActionProof,
) -> Result<()> {
    let result = account::delete_account(&database, reset.context(), &proof).await;
    emit_if_reset(&app, &result);
    if result.is_ok() {
        app.state::<EventsService>().stop();
    }
    result
}

#[tauri::command]
pub async fn account_devices(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
) -> Result<Vec<AccountDevice>> {
    let result = account::account_devices(&database, reset.context()).await;
    emit_if_reset(&app, &result);
    result
}

#[tauri::command]
pub async fn remove_account_device(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    id: String,
) -> Result<()> {
    let result = account::remove_account_device(&database, reset.context(), &id).await;
    emit_if_reset(&app, &result);
    result
}

#[tauri::command]
pub async fn restore_account_device(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    id: String,
) -> Result<()> {
    let result = account::restore_account_device(&database, reset.context(), &id).await;
    emit_if_reset(&app, &result);
    result
}

/// The code, sent to the account's own email, that an account with no password gives to confirm an action (D100).
#[tauri::command]
pub async fn request_action_code(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
) -> Result<()> {
    let result = account::request_action_code(&database, reset.context()).await;
    emit_if_reset(&app, &result);
    result
}

#[tauri::command]
pub async fn two_factor_setup(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
) -> Result<TwoFactorSetup> {
    let result = account::two_factor_setup(&database, reset.context()).await;
    emit_if_reset(&app, &result);
    result
}

#[tauri::command]
pub async fn two_factor_enable(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    code: String,
    proof: ActionProof,
) -> Result<Vec<String>> {
    let result = account::two_factor_enable(&database, reset.context(), &code, &proof).await;
    emit_if_reset(&app, &result);
    result
}

#[tauri::command]
pub async fn two_factor_disable(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    password: Option<String>,
    code: String,
) -> Result<()> {
    let result =
        account::two_factor_disable(&database, reset.context(), password.as_deref(), &code).await;
    emit_if_reset(&app, &result);
    result
}

#[tauri::command]
pub async fn regenerate_backup_codes(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    password: Option<String>,
    code: String,
) -> Result<Vec<String>> {
    let result =
        account::regenerate_backup_codes(&database, reset.context(), password.as_deref(), &code)
            .await;
    emit_if_reset(&app, &result);
    result
}

/// "Dejar este dispositivo a otra persona" (D81): this always resets the device,
/// whether or not the server could be told, so the UI reloads unconditionally.
#[tauri::command]
pub async fn give_device_away(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
) -> Result<()> {
    let result = account::give_device_away(&database, reset.context()).await;
    app.state::<EventsService>().stop();
    let _ = app.emit("device-reset", ());
    result
}

// --- The cloud (D84, hito 7.2) ----------------------------------------------

/// Local only: for each library, whether it is in the cloud and whether it could go.
#[tauri::command]
pub fn cloud_status(database: State<'_, Database>) -> Result<Vec<LibraryCloudStatus>> {
    cloud::status(&database)
}

/// Takes a library to the cloud of this account (needs «Can share», D73).
#[tauri::command]
pub async fn upload_to_cloud(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    library_id: String,
    autolink: bool,
) -> Result<LibraryReport> {
    let result = match HttpCloud::connect(&database) {
        Ok(api) => cloud::upload_with(&database, &api, &library_id, autolink).await,
        Err(error) => Err(error),
    };
    let result = account::react_to_account_error(&database, reset.context(), result).await;
    emit_if_reset(&app, &result);
    result
}

/// Gives the cloud the power to link new devices of the library (D76), or takes it away.
#[tauri::command]
pub async fn set_cloud_power(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    library_id: String,
    on: bool,
) -> Result<()> {
    let result = match HttpCloud::connect(&database) {
        Ok(api) => cloud::set_cloud_power(&database, &api, &library_id, on).await,
        Err(error) => Err(error),
    };
    let result = account::react_to_account_error(&database, reset.context(), result).await;
    emit_if_reset(&app, &result);
    result
}

/// Libraries where this device is another person's than the signed-in account, not answered yet (D76).
#[tauri::command]
pub fn cloud_other_owners(
    database: State<'_, Database>,
) -> Result<Vec<cloud::other_owner::OtherOwner>> {
    cloud::other_owner::pending(&*database.lock()?)
}

/// «Leave it as it is»: the question about `library_id` is not asked again.
#[tauri::command]
pub fn answer_other_owner(database: State<'_, Database>, library_id: String) -> Result<()> {
    cloud::other_owner::answer(&*database.lock()?, &library_id)
}

/// Syncs every library of the account with the cloud, one report per library. Then, in the background, the covers
/// their editions point to (D82); `covers-arrived` tells the UI to show the ones that came.
#[tauri::command]
pub async fn sync_cloud(
    app: AppHandle,
    database: State<'_, Database>,
    covers: State<'_, Covers>,
    reset: State<'_, ResetDeps>,
) -> Result<Vec<LibraryReport>> {
    let result = match HttpCloud::connect(&database) {
        Ok(api) => cloud::sync_all(&database, &api).await,
        Err(error) => Err(error),
    };
    let result = account::react_to_account_error(&database, reset.context(), result).await;
    emit_if_reset(&app, &result);
    if result.is_ok() {
        let (database, covers, app) = (
            database.inner().clone(),
            covers.inner().clone(),
            app.clone(),
        );
        tauri::async_runtime::spawn(async move {
            let Ok(api) = HttpCloud::connect(&database) else {
                return;
            };
            if cloud::exchange_covers(&database, &api, &covers)
                .await
                .is_ok_and(|received| received > 0)
            {
                let _ = app.emit("covers-arrived", ());
            }
        });
    }
    result
}

/// The app is in the foreground again: Android may have killed the live connection,
/// so a retry that is waiting for its backoff goes now.
#[tauri::command]
pub fn cloud_events_wake(events: State<'_, EventsService>) {
    events.wake();
}

/// For the first render: is the live connection up, and does the server have one?
#[tauri::command]
pub fn cloud_connection(events: State<'_, EventsService>) -> Connection {
    events.connection()
}

// --- Invitations (D43, milestone 7.6.2) --------------------------------------

/// Can this library invite now, and if not, why. Local, plus asking the server whether it makes invitations; no
/// session needed, so a signed-out device gets its reason without a request.
#[tauri::command]
pub async fn invite_status(
    database: State<'_, Database>,
    library_id: String,
) -> Result<InviteStatus> {
    let api = HttpCloud::anonymous(&database)?;
    invitations::invite_status(&database, &api, &library_id).await
}

/// Invites somebody to a library. For a new person the person is created first, always as a reader.
#[tauri::command]
pub async fn invite_to_library(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    library_id: String,
    request: InviteRequest,
) -> Result<InviteResult> {
    let result = match HttpCloud::connect(&database) {
        Ok(api) => invitations::invite(&database, &api, &library_id, request).await,
        Err(error) => Err(error),
    };
    let result = account::react_to_account_error(&database, reset.context(), result).await;
    emit_if_reset(&app, &result);
    result
}

/// The invitations of a library, for whoever administers it.
#[tauri::command]
pub async fn list_invitations(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    library_id: String,
) -> Result<Vec<InvitationItem>> {
    let result = match HttpCloud::connect(&database) {
        Ok(api) => invitations::list_invitations(&database, &api, &library_id).await,
        Err(error) => Err(error),
    };
    let result = account::react_to_account_error(&database, reset.context(), result).await;
    emit_if_reset(&app, &result);
    result
}

#[tauri::command]
pub async fn revoke_invitation(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    library_id: String,
    invitation_id: String,
) -> Result<()> {
    let result = match HttpCloud::connect(&database) {
        Ok(api) => {
            invitations::revoke_invitation(&database, &api, &library_id, &invitation_id).await
        }
        Err(error) => Err(error),
    };
    let result = account::react_to_account_error(&database, reset.context(), result).await;
    emit_if_reset(&app, &result);
    result
}

/// Opens an invitation link (pasted, or handed over by the system when it arrives that way), with no session:
/// says what it is for and keeps its token until the account that accepts it is signed in. A link of another
/// server than this device's is refused without asking anything.
#[tauri::command]
pub async fn open_invitation(
    database: State<'_, Database>,
    link: String,
) -> Result<OpenedInvitation> {
    let api = HttpCloud::anonymous(&database)?;
    invitations::open(&database, &api, &link).await
}

/// Starts «Continuar con Google» (D77): asks the account's server for an attempt and opens the system browser at
/// the address it gives. The way back arrives by `kosmotheka://auth/google` and is read by `complete_google_return`.
#[tauri::command]
pub async fn start_google(
    app: AppHandle,
    database: State<'_, Database>,
    locale: String,
) -> Result<()> {
    let url = account::google::start(&database, &locale).await?;
    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_url(url, None::<&str>)
        .map_err(|_| Error::account("google_failed"))
}

/// «Cancel» while the browser is open: a way back that comes later is not an attempt of this app any more.
#[tauri::command]
pub fn cancel_google(database: State<'_, Database>) -> Result<()> {
    account::google::cancel(&database)
}

/// The way back from Google, if one arrived: signs this device in, or says why not. `None` when nothing is waiting or
/// it is not for the attempt this app started (a link somebody else made), which asks no server anything.
#[tauri::command]
pub async fn complete_google_return(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    arrivals: State<'_, Arrivals>,
) -> Result<Option<AccountUser>> {
    let Some(back) = arrivals.take_google() else {
        return Ok(None);
    };
    let result = account::google::complete(&database, reset.context(), back).await;
    emit_if_reset(&app, &result);
    if matches!(result, Ok(Some(_))) {
        app.state::<EventsService>().start();
    }
    result
}

/// The second factor's code, for a way back that was waiting for it.
#[tauri::command]
pub async fn finish_google(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
    totp_code: String,
) -> Result<AccountUser> {
    let result = account::google::finish(&database, reset.context(), Some(totp_code)).await;
    emit_if_reset(&app, &result);
    if result.is_ok() {
        app.state::<EventsService>().start();
    }
    result
}

/// Looks at the link the system handed to the app, if one is waiting (milestone 7.6.4): the same road as a link
/// pasted by hand, and no more than looking. The link never reaches the screen, only what it says.
#[tauri::command]
pub async fn open_arrived_link(
    database: State<'_, Database>,
    arrivals: State<'_, Arrivals>,
) -> Result<Option<OpenedInvitation>> {
    let api = HttpCloud::anonymous(&database)?;
    links::attend(&arrivals, &database, &api).await
}

/// The link that is waiting for a session, if one is and has not expired.
#[tauri::command]
pub fn pending_invitation(database: State<'_, Database>) -> Result<PendingInvitation> {
    invitations::pending_invitation(&database)
}

/// «Not this one»: forgets the link that was waiting.
#[tauri::command]
pub fn discard_pending_invitation(database: State<'_, Database>) -> Result<()> {
    invitations::discard_pending_invitation(&database)
}

/// Accepts the link that is waiting, with the signed-in account, and syncs so the library comes down.
#[tauri::command]
pub async fn accept_invitation(
    app: AppHandle,
    database: State<'_, Database>,
    reset: State<'_, ResetDeps>,
) -> Result<Accepted> {
    let result = match HttpCloud::connect(&database) {
        Ok(api) => invitations::accept_pending(&database, &api).await,
        Err(error) => Err(error),
    };
    let result = account::react_to_account_error(&database, reset.context(), result).await;
    emit_if_reset(&app, &result);
    result
}
