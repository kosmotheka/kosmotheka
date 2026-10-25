import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { join } from "@tauri-apps/api/path";
import type { Language } from "@/i18n";

// These types mirror the Rust structs in crates/domain and src-tauri/src/catalog.rs.

export const formats = [
  "hardcover",
  "paperback",
  "pocket",
  "ebook",
  "pdf",
  "audiobook",
  "single_issue",
  "box",
  "dvd",
  "bluray",
  "uhd",
  "physical",
  "digital",
  "vinyl",
  "cd",
  "cassette",
  "other",
] as const;
export type Format = (typeof formats)[number];

export const kinds = [
  "book",
  "comic",
  "boardgame",
  "film",
  "tv_series",
  "videogame",
  "music",
  "other",
] as const;
export type Kind = (typeof kinds)[number];

export const contributorRoles = [
  "author",
  "writer",
  "illustrator",
  "designer",
  "director",
  "artist",
  "studio",
] as const;
export type ContributorRole = (typeof contributorRoles)[number];

interface ContributorInput {
  name: string;
  role: ContributorRole;
}

export interface EditionInput {
  title: string;
  subtitle: string | null;
  authors: string[];
  publisher: string | null;
  publishedYear: number | null;
  language: string | null;
  isbn: string | null;
  /** Remote cover to download on save (must be on a known cover host). */
  coverUrl: string | null;
  /** A cover of one's own already stored on this device (`storeOwnCover`); it wins over `coverUrl`. */
  ownCover?: string | null;
  googleBooksId: string | null;
  openLibraryId: string | null;
  kind?: Kind;
  /** Contributors in any role other than the book's authors (`authors` means `author`). */
  contributors?: ContributorInput[];
  /** EAN/UPC, for what has no ISBN. */
  barcode?: string | null;
  number?: string | null;
  playersMin?: number | null;
  playersMax?: number | null;
  durationMinutes?: number | null;
  minAge?: number | null;
  seasons?: number | null;
  genres?: string[];
  /** Tag names; found or created case-insensitively. */
  tags?: string[];
  notes?: string | null;
}

export interface CopyInput {
  owner: string | null;
  format: Format;
  quantity: number;
  location: string | null;
  acquiredOn: string | null;
  /** A video game copy's platform: belongs to the copy, not the edition. */
  platform?: string | null;
  /** A TV series copy's held seasons, as free text ("1-2", "1-3, 5"). */
  seasons?: string | null;
}

export interface NewBook extends EditionInput {
  copy: CopyInput;
}

export interface CreatedBook {
  editionId: string;
  existingEdition: boolean;
  /** Saved, but the cover could not be downloaded. */
  coverFailed: boolean;
}

export interface BookSummary {
  editionId: string;
  /** The library the book is in; searches across libraries mix them. */
  libraryId: string;
  kind: Kind;
  title: string;
  subtitle: string | null;
  authors: string[];
  publisher: string | null;
  publishedYear: number | null;
  isbn13: string | null;
  cover: string | null;
  copyCount: number;
  formats: Format[];
  owners: string[];
  /** This device's person's status for the work (D68). */
  myStatus: MarkStatus | null;
  myFavorite: boolean;
  myWished: boolean;
  /** Names of the members who wished the work (D61): this device's person first,
   * as `"me"` if they wished it, the rest sorted by name. */
  wishedBy: string[];
  tags: string[];
}

type FieldErrorCode =
  | "required"
  | "out_of_range"
  | "invalid_date"
  | "isbn_length"
  | "isbn_character"
  | "isbn_checksum"
  | "isbn_prefix"
  | "isbn_in_use"
  | "invalid_cover_url"
  | "invalid_format"
  | "name_in_use"
  | "invalid_barcode"
  | "barcode_in_use"
  | "invalid_genre"
  | "self_reference";

export interface FieldError {
  field: string;
  code: FieldErrorCode;
}

export type CommandError =
  | { kind: "validation"; errors: FieldError[] }
  | { kind: "backup"; reason: string }
  | { kind: "sync"; reason: string }
  | {
      kind: "permission";
      reason: "not_allowed" | "last_admin" | "request_resolved" | "cannot_share";
    }
  | { kind: "internal"; message: string }
  | { kind: "account"; reason: string; retryAfter?: number | null }
  | { kind: "cloud"; reason: CloudErrorReason };

export function isCommandError(value: unknown): value is CommandError {
  return typeof value === "object" && value !== null && "kind" in value;
}

export function createBook(book: NewBook): Promise<CreatedBook> {
  return invoke<CreatedBook>("create_book", { book });
}

/** Whether a work's edition has copies (D61): `missing` is catalogued without owning it yet. */
export type Holding = "owned" | "missing";

export interface BookFilter {
  query?: string;
  kind?: Kind;
  format?: Format;
  /** `owned` = has copies, `missing` = has none (D61); omitted matches everything. */
  holding?: Holding;
  /** Owner name; matches duplicates created on other devices too. */
  owner?: string;
  location?: string;
  /** My status, any of them; `none` matches works I have not marked (D68). */
  myStatuses?: (MarkStatus | "none")[];
  myFavorites?: boolean;
  /** Works I wish I had (D61). */
  myWished?: boolean;
  /** Works someone other than me wishes. */
  othersWished?: boolean;
  /** A work matches if it has any of these (ignored when empty). */
  genres?: string[];
  /** Tag ids; a work matches if it has any of these (ignored when empty). */
  tags?: string[];
}

/** The active library's books, or every library's with `everywhere`. */
export function listBooks(filter: BookFilter = {}, everywhere = false): Promise<BookSummary[]> {
  return invoke<BookSummary[]>("list_books", { filter, everywhere });
}

export interface CatalogOptions {
  authors: string[];
  publishers: string[];
  owners: string[];
  locations: string[];
}

export function catalogOptions(): Promise<CatalogOptions> {
  return invoke<CatalogOptions>("catalog_options");
}

export interface HistoryEntry {
  operationId: string;
  intent: string;
  /** Milliseconds since the Unix epoch. */
  timestamp: number;
}

export function bookHistory(editionId: string): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>("book_history", { editionId });
}

export interface CopyDetail {
  id: string;
  owner: string | null;
  format: Format;
  quantity: number;
  location: string | null;
  acquiredOn: string | null;
  /** A video game copy's platform: belongs to the copy, not the edition. */
  platform: string | null;
  /** A TV series copy's held seasons, as free text ("1-2", "1-3, 5"). */
  seasons: string | null;
  /** This device may delete it; otherwise it can only ask (D73). */
  canDelete: boolean;
}

interface ContributorDetail {
  name: string;
  role: ContributorRole;
}

export interface BookDetail {
  editionId: string;
  workId: string;
  kind: Kind;
  title: string;
  subtitle: string | null;
  authors: string[];
  /** Every role but author, ordered by role then position. */
  contributors: ContributorDetail[];
  publisher: string | null;
  publishedYear: number | null;
  language: string | null;
  isbn13: string | null;
  cover: string | null;
  googleBooksId: string | null;
  openLibraryId: string | null;
  barcode: string | null;
  notes: string | null;
  number: string | null;
  playersMin: number | null;
  playersMax: number | null;
  /** Boardgame or film, whichever the edition's kind is; never both at once. */
  durationMinutes: number | null;
  minAge: number | null;
  seasons: number | null;
  copies: CopyDetail[];
  /** This device may delete the whole book; otherwise it can only ask. */
  canDelete: boolean;
  /** Sorted case-insensitively. */
  genres: string[];
  /** Names, sorted case-insensitively. */
  tags: string[];
  /** Names of the members who wished the work (D61): this device's person first,
   * as `"me"` if they wished it, the rest sorted by name. */
  wishedBy: string[];
  /** The series this work belongs to. */
  series: WorkSeries[];
  /** Works this one requires (an expansion needs its base game). */
  requires: RelatedWork[];
  /** Works that require this one. */
  requiredBy: RelatedWork[];
}

export interface WorkSeries {
  seriesId: string;
  name: string;
  ordered: boolean;
  position: number | null;
  /** What the series holds counting its gaps, for «2 de 6». */
  total: number;
}

/** A work related to another (D61): its first edition and how many copies it has,
 * so the UI can say "you have the expansion but not the base game". */
export interface RelatedWork {
  workId: string;
  editionId: string | null;
  title: string;
  copies: number;
}

export function getBook(editionId: string): Promise<BookDetail> {
  return invoke<BookDetail>("get_book", { editionId });
}

export interface TagSummary {
  id: string;
  name: string;
  /** Distinct works this tag is linked to. */
  works: number;
}

export function listTags(): Promise<TagSummary[]> {
  return invoke<TagSummary[]>("list_tags");
}

export function renameTag(id: string, name: string): Promise<void> {
  return invoke("rename_tag", { id, name });
}

export function deleteTag(id: string): Promise<void> {
  return invoke("delete_tag", { id });
}

export function genresOfKind(kind: Kind): Promise<string[]> {
  return invoke<string[]>("genres_of_kind", { kind });
}

export function updateEdition(
  editionId: string,
  edition: EditionInput,
): Promise<{ coverFailed: boolean }> {
  return invoke("update_edition", { editionId, edition });
}

export function deleteBook(editionId: string): Promise<void> {
  return invoke("delete_book", { editionId });
}

export function addCopy(editionId: string, copy: CopyInput): Promise<string> {
  return invoke<string>("add_copy", { editionId, copy });
}

export function updateCopy(copyId: string, copy: CopyInput): Promise<void> {
  return invoke("update_copy", { copyId, copy });
}

export function deleteCopy(copyId: string): Promise<void> {
  return invoke("delete_copy", { copyId });
}

/** What asking became: kept for the administrators, or simply done (D73). */
export interface Asked {
  requested: boolean;
}

export function requestBook(book: NewBook, note: string | null): Promise<Asked> {
  return invoke<Asked>("request_book", { book, note });
}

/** Catalogs a work with no copy yet (D61); editors and admins only. */
export function addMissing(edition: EditionInput): Promise<CreatedBook> {
  return invoke<CreatedBook>("add_missing", { edition });
}

export function requestMissing(edition: EditionInput, note: string | null): Promise<Asked> {
  return invoke<Asked>("request_missing", { edition, note });
}

/** «Ask to add a copy» of a work the reader is missing (D73). */
export function requestAddCopy(
  editionId: string,
  copy: CopyInput,
  note: string | null,
): Promise<Asked> {
  return invoke<Asked>("request_add_copy", { editionId, copy, note });
}

export function requestDeleteCopy(copyId: string, note: string | null): Promise<Asked> {
  return invoke<Asked>("request_delete_copy", { copyId, note });
}

export function requestDeleteBook(editionId: string, note: string | null): Promise<Asked> {
  return invoke<Asked>("request_delete_book", { editionId, note });
}

export type Source =
  | "bne"
  | "library_of_congress"
  | "google_books"
  | "open_library"
  | "inventaire"
  | "music_brainz"
  | "wikidata"
  | "game_upc"
  | "tmdb"
  | "rawg"
  | "comic_vine"
  | "board_game_geek"
  | "rpg_geek";

export interface LookupResult {
  isbn13: string;
  found: boolean;
  title: string | null;
  subtitle: string | null;
  authors: string[];
  publisher: string | null;
  publishedYear: number | null;
  language: string | null;
  coverUrl: string | null;
  googleBooksId: string | null;
  openLibraryId: string | null;
  sources: Source[];
  warnings: { source: Source; kind: "network" | "quota" | "bad_key" | "unexpected" }[];
  existingEditionId: string | null;
}

export function lookupIsbn(isbn: string): Promise<LookupResult> {
  return invoke<LookupResult>("lookup_isbn", { isbn });
}

/** The game an expansion needs: kept on the candidate, though nothing creates the relation yet (D61). */
interface CandidateExpands {
  externalId: string;
  title: string;
}

interface CandidateContributor {
  name: string;
  role: ContributorRole;
}

/** One result a person can pick to prefill the add form; fields match `EditionInput`. */
export interface Candidate {
  kind: Kind;
  source: Source;
  externalId: string | null;
  title: string;
  subtitle: string | null;
  authors: string[];
  /** Contributors in any role other than author (`authors` means `author`). */
  contributors: CandidateContributor[];
  publisher: string | null;
  publishedYear: number | null;
  language: string | null;
  coverUrl: string | null;
  /** ISBN-13. */
  isbn: string | null;
  barcode: string | null;
  number: string | null;
  playersMin: number | null;
  playersMax: number | null;
  durationMinutes: number | null;
  minAge: number | null;
  seasons: number | null;
  platform: string | null;
  /** A suggested copy format, when the provider knows it. */
  format: Format | null;
  /** The base game, when this one is an expansion of it (BoardGameGeek). */
  expands: CandidateExpands | null;
  /** Set when this candidate's ISBN or barcode already exists in the active library. */
  existingEditionId: string | null;
}

export interface SearchResult {
  candidates: Candidate[];
  warnings: { source: Source; kind: "network" | "quota" | "bad_key" | "unexpected" }[];
}

export function searchWorks(kind: Kind, query: string, language?: string): Promise<SearchResult> {
  return invoke<SearchResult>("search_works", { kind, query, language });
}

/** The shelf photo: the texts read on its spines worth searching for. Only on a phone, like the scanner. */
export function shelfQueries(photo: string): Promise<string[]> {
  return invoke<string[]>("shelf_queries", { photo });
}

/** One text read on a shelf, searched as if typed: the work whose title is what was read, or null. */
export function shelfMatch(kind: Kind, read: string, language?: string): Promise<Candidate | null> {
  return invoke<Candidate | null>("shelf_match", { kind, read, language });
}

export interface CodeResult {
  kind: Kind | null;
  candidates: Candidate[];
  warnings: { source: Source; kind: "network" | "quota" | "bad_key" | "unexpected" }[];
  existingEditionId: string | null;
  /** A number written like an ISBN whose check digit is wrong: nothing was asked anywhere (decided in Rust). */
  invalidIsbn: boolean;
  /** The ISBN-13 of a valid ISBN, found or not. */
  isbn: string | null;
  /** How sure GameUPC was of this barcode, when it was asked (D62): with `choose` or `unknown`, the game picked is sent back. */
  gameUpc: "verified" | "choose" | "unknown" | null;
}

export function lookupCode(code: string): Promise<CodeResult> {
  return invoke<CodeResult>("lookup_code", { code });
}

/** Tells GameUPC, through the account's server, the board game a person chose for a barcode it was not sure of (D62). */
export function gameUpcChoice(code: string, externalId: string): Promise<void> {
  return invoke("gameupc_choice", { code, externalId });
}

export interface Settings {
  apiKeys: Record<string, string>;
  disabledSources: string[];
  /** Ask Google Books through the account's server when there is no key of their own (7.8, D83). */
  googleBooksViaServer: boolean;
}

export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

/** Which catalogues (`google_books`, `tmdb`) the server of this device's session looks up for its accounts; none without a session. */
export function cloudMetadataOffers(): Promise<string[]> {
  return invoke<string[]>("cloud_metadata_offers");
}

/** What the server of the session asks for its accounts, and the part this device has not been told about (D91). */
export interface ServerServices {
  offered: string[];
  new: string[];
}

export function serverServices(): Promise<ServerServices> {
  return invoke<ServerServices>("server_services");
}

export function markServerServicesSeen(services: string[]): Promise<void> {
  return invoke("mark_server_services_seen", { services });
}

export function saveSettings(settings: Settings): Promise<void> {
  return invoke("save_settings", { settings });
}

let coversDir: Promise<string> | null = null;

/** URL the webview can load for a stored cover file. */
export async function coverSrc(file: string): Promise<string> {
  coversDir ??= invoke<string>("covers_dir").catch((error: unknown) => {
    coversDir = null; // let the next call try again
    throw error;
  });
  // join() uses the platform's separator, which the asset protocol scope expects on Windows.
  return convertFileSrc(await join(await coversDir, file));
}

export const externalLinks = {
  googleBooks: (id: string) => `https://books.google.com/books?id=${encodeURIComponent(id)}`,
  openLibrary: (id: string) => `https://openlibrary.org/books/${encodeURIComponent(id)}`,
};

export interface BackupSummary {
  libraries: number;
  books: number;
  copies: number;
  covers: number;
}

/** What a restore did, library by library. */
export interface RestoreSummary {
  restored: { id: string; name: string; books: number; copies: number; sameId: boolean }[];
  /** Libraries this device already had with content; left as they were. */
  skipped: string[];
  covers: number;
}

/**
 * Backs up the active library, or `library` if given. With a `passphrase` the file is
 * encrypted (D86); without one it is a plain zip anyone can read.
 */
export function exportBackup(
  path: string,
  library?: string,
  passphrase?: string,
): Promise<BackupSummary> {
  return invoke<BackupSummary>("export_backup", { path, library, passphrase });
}

/** Backs up every library on this device in one file. */
export function exportDeviceBackup(path: string, passphrase?: string): Promise<BackupSummary> {
  return invoke<BackupSummary>("export_device_backup", { path, passphrase });
}

/**
 * Fails with the backup reason `passphrase_needed` for an encrypted file given no
 * passphrase, and `wrong_passphrase` for a wrong one; either way nothing is restored.
 */
export function restoreBackup(path: string, passphrase?: string): Promise<RestoreSummary> {
  return invoke<RestoreSummary>("restore_backup", { path, passphrase });
}

export function exportCsv(path: string): Promise<number> {
  return invoke<number>("export_csv", { path });
}

export function exportLicenses(path: string): Promise<void> {
  return invoke("export_licenses", { path });
}

export interface CsvImportSummary {
  booksCreated: number;
  copiesAdded: number;
  rejected: { line: number; errors: FieldError[] }[];
}

export function importCsv(path: string): Promise<CsvImportSummary> {
  return invoke<CsvImportSummary>("import_csv", { path });
}

export interface KnownDevice {
  deviceId: string;
  name: string;
  pairedAt: string;
  lastSyncAt: string | null;
}

export interface LanStatus {
  deviceId: string;
  deviceName: string;
  servingPort: number | null;
  addresses: string[];
  pairingOpen: boolean;
  /** Devices this one syncs with (it connects to them). */
  servers: KnownDevice[];
  /** Devices that sync with this one. */
  clients: KnownDevice[];
}

export interface SyncReport {
  received: { received: number; known: number; rejected: number; skippedChanges: number };
  sent: number;
  /** This device took up a library the other one started sharing. */
  takenUp: boolean;
  /** Libraries removed here because the other device withdrew access (D71). */
  removed: string[];
  /** Libraries that arrived linked to a cloud on a server that is not the official one. */
  foreignClouds: { library: string; name: string; cloudServer: string }[];
}

/** A paired device and the libraries that go to it. */
export interface DeviceSharing extends KnownDevice {
  libraries: string[];
}

export function lanStatus(): Promise<LanStatus> {
  return invoke<LanStatus>("lan_status");
}

export function setDeviceName(name: string): Promise<void> {
  return invoke("set_device_name", { name });
}

/** Opens a pairing window; whoever pairs through it receives `libraries`. */
export function openPairing(
  libraries: string[],
): Promise<{ code: string; pin: string; expiresInSeconds: number }> {
  return invoke("open_pairing", { libraries });
}

export function closePairing(): Promise<void> {
  return invoke("close_pairing");
}

/** Pairs with the device that showed `code` and shares `libraries` with it. */
export function pairWith(
  code: string,
  libraries: string[],
): Promise<{ deviceId: string; name: string }> {
  return invoke("pair_with", { code, libraries });
}

/** A device nearby with a pairing window open (D88): before the number, all it says is its name. */
export interface NearbyDevice {
  deviceId: string;
  name: string;
  address: string;
}

export function findPairingDevices(): Promise<NearbyDevice[]> {
  return invoke<NearbyDevice[]>("find_pairing_devices");
}

/** Pairs with the device at `address` with the six digits it shows (D88). */
export function pairWithPin(
  address: string,
  pin: string,
  libraries: string[],
): Promise<{ deviceId: string; name: string }> {
  return invoke("pair_with_pin", { address, pin, libraries });
}

export function sharingOverview(): Promise<DeviceSharing[]> {
  return invoke<DeviceSharing[]>("sharing_overview");
}

/** Shares a library with a paired device, or withdraws its access (D71). */
export function setSharing(library: string, deviceId: string, shared: boolean): Promise<void> {
  return invoke("set_sharing", { library, deviceId, shared });
}

export function syncWith(deviceId: string): Promise<SyncReport> {
  return invoke<SyncReport>("sync_with", { deviceId });
}

export function forgetDevice(deviceId: string): Promise<void> {
  return invoke("forget_device", { deviceId });
}

/** Translation key for a failed sync command. */
export function syncErrorKey(error: unknown): string {
  if (isCommandError(error) && error.kind === "sync") return `devices.errors.${error.reason}`;
  return "devices.errors.peer_error";
}

export interface IsbnCheck {
  isbn13: string;
  /** Editions with at least one copy. */
  books: BookSummary[];
  /** Editions with no copy yet, in "Missing" (D61). */
  missing: BookSummary[];
}

/** Local only: works without coverage. */
export function checkIsbn(isbn: string): Promise<IsbnCheck> {
  return invoke<IsbnCheck>("check_isbn", { isbn });
}

let mobile: Promise<boolean> | null = null;

export function isMobile(): Promise<boolean> {
  mobile ??= invoke<boolean>("is_mobile").catch(() => false);
  return mobile;
}

/** Whether the fingerprint/face lock is on (D78). Per-device, never synced. */
export function appLockEnabled(): Promise<boolean> {
  return invoke<boolean>("app_lock_enabled");
}

/** The version of the app that is installed, as its package says it. */
export function appVersion(): Promise<string> {
  return getVersion();
}

/** The version of the first-run welcome this device has seen (D91); 0 when none. */
export function welcomeSeen(): Promise<number> {
  return invoke<number>("welcome_seen");
}

/** The text of a line in each interface language that has one (English always does), and its switch, if any. */
export type WhatsNewLine = Partial<Record<Language, string>> & {
  en: string;
  /** The setting this change is tied to, with how it is now. */
  switch: { key: string; on: boolean } | null;
};

export interface WhatsNew {
  /** What changes what leaves the device: shown first, each with its switch. */
  switches: WhatsNewLine[];
  /** Newest first; `next` is a build between releases. */
  versions: { version: string; lines: WhatsNewLine[] }[];
}

/** What this device still has to see after an update; empty on a new install. */
export function whatsNewPending(): Promise<WhatsNew> {
  return invoke<WhatsNew>("whats_new_pending");
}

/** The latest versions' news, to read again. */
export function whatsNewRecent(): Promise<WhatsNew> {
  return invoke<WhatsNew>("whats_new_recent");
}

/** Marks the installed version's news as seen. */
export function setWhatsNewSeen(): Promise<void> {
  return invoke("set_whats_new_seen");
}

/** Turns on or off the setting a news line is tied to. */
export function setWhatsNewSwitch(key: string, on: boolean): Promise<void> {
  return invoke("set_whats_new_switch", { key, on });
}

/** The name this device's person goes by (D97); `null` until the welcome has it. */
export function personName(): Promise<string | null> {
  return invoke<string | null>("person_name");
}

/** The welcome's first question: with the name, «Mi biblioteca» is founded with this person as its administrator. */
export function beginWithName(name: string): Promise<void> {
  return invoke("begin_with_name", { name });
}

/** Changes the name for the libraries created from now on; the ones that exist keep theirs. */
export function setPersonName(name: string): Promise<string> {
  return invoke<string>("set_person_name", { name });
}

export function setWelcomeSeen(version: number): Promise<void> {
  return invoke("set_welcome_seen", { version });
}

export function setAppLock(enabled: boolean): Promise<void> {
  return invoke("set_app_lock", { enabled });
}

/** Minutes the app may stay in the background before it locks again (D78). */
export function appLockAfter(): Promise<number> {
  return invoke<number>("app_lock_after");
}

export function setAppLockAfter(minutes: number): Promise<void> {
  return invoke("set_app_lock_after", { minutes });
}

// --- Libraries -----------------------------------------------------------------

export interface LibrarySummary {
  id: string;
  /** Empty for a library never named: show `nav.myLibrary` instead. */
  name: string;
  works: number;
  active: boolean;
  created: { device: string | null; at: number } | null;
  /** This device may take it to other devices (D73). */
  canShare: boolean;
}

export function listLibraries(): Promise<LibrarySummary[]> {
  return invoke<LibrarySummary[]>("list_libraries");
}

/** Creates a library and makes it active. Returns its id. */
export function createLibrary(name: string): Promise<string> {
  return invoke<string>("create_library", { name });
}

export function renameLibrary(id: string, name: string): Promise<void> {
  return invoke("rename_library", { id, name });
}

export function setActiveLibrary(id: string): Promise<void> {
  return invoke("set_active_library", { id });
}

/** Deletes it on every device it is shared with. Returns the library now active. */
export function deleteLibrary(id: string): Promise<string> {
  return invoke<string>("delete_library", { id });
}

/**
 * Copies its catalogue into a new library this device founds and administers, without
 * the other people's roles, devices or history: the last resort when nobody can administer it (D58).
 * Returns the new library's id, which becomes the active one.
 */
export function duplicateLibrary(id: string, name: string): Promise<string> {
  return invoke<string>("duplicate_library", { id, name });
}

/**
 * Removes it from this device only. Returns the library now active. When no paired device has it, the cloud has
 * to confirm it keeps it for this account; otherwise it fails with `OnlyOnThisDevice`.
 */
export function leaveLibrary(id: string): Promise<string> {
  return invoke<string>("leave_library", { id });
}

/**
 * «Leave the library» for good (D102): this device's person withdraws her own access in the active library. The
 * library goes from this device once the operation has reached another one.
 */
export function leaveLibraryForGood(): Promise<void> {
  return invoke("leave_library_for_good");
}

/** Whether the cloud of the signed-in account offers this library right now. Any failure is `false`. */
export function libraryInCloud(id: string): Promise<boolean> {
  return invoke<boolean>("library_in_cloud", { id });
}

/** A library this device removed by its own choice, which the cloud may hand over again. */
export interface LeftLibrary {
  id: string;
  /** Empty for a library never named. */
  name: string;
  cloudLibrary: string;
  cloudServer: string;
}

export function leftLibraries(): Promise<LeftLibrary[]> {
  return invoke<LeftLibrary[]>("left_libraries");
}

/** Brings one of `leftLibraries` down from the cloud again. Nothing else can come back this way. */
export function redownloadLibrary(id: string): Promise<CloudLibraryReport> {
  return invoke<CloudLibraryReport>("redownload_library", { id });
}

export function libraryDevices(id: string): Promise<KnownDevice[]> {
  return invoke<KnownDevice[]>("library_devices", { id });
}

/** Moves a book of the active library to `to`. Returns its new edition id. */
export function moveBook(editionId: string, to: string): Promise<string> {
  return invoke<string>("move_book", { editionId, to });
}

// --- People and roles (D55, D73), mirrors src-tauri/src/people.rs --------------

export type Role = "admin" | "editor" | "reader";
export const roles: Role[] = ["admin", "editor", "reader"];

export interface PersonDevice {
  id: string;
  name: string;
  /** The device this app runs on. */
  thisDevice: boolean;
  /** Removing it would leave nobody who administers and can act: Rust refuses (SEG-036). */
  lastDeviceOfLastAdmin: boolean;
  /** The operation that said whose it is was signed by the cloud (D76, D96). */
  linkedByCloud: boolean;
  /** The person whose device linked it, when it was neither the cloud nor the device itself. */
  linkedBy: string | null;
  /** When it last signed anything in this library (ms since 1970): its own registration at least. */
  lastChange: number | null;
}

export interface Person {
  id: string;
  name: string;
  role: Role | null;
  canShare: boolean;
  /** Sent away (D98): no access, but they stay in the catalog with their books. */
  accessWithdrawn: boolean;
  /** The last administrator who can act: not demoted nor sent away (D58, SEG-036). */
  lastAdmin: boolean;
  /** An account is linked to this person (D76); which one is never shown. */
  hasAccount: boolean;
  /** Whether they can be invited, from what this device knows: only a reader or editor with no account who still has access (D43). */
  invitable: "email" | "none";
  devices: PersonDevice[];
  /** Other libraries on this device where the same person is. */
  elsewhere: string[];
}

export interface People {
  myRole: Role | null;
  people: Person[];
  /** Devices that belong to nobody: they count as readers. */
  unassigned: PersonDevice[];
  /** Only one person administers the library (D58). */
  loneAdmin: boolean;
  /** This device's earlier installs: same person, same name, nothing signed since this one came (D81). */
  earlierInstalls: PersonDevice[];
}

/** A person picked on screen: an existing one (`id`) or somebody new (`name`). */
export interface PersonPick {
  id: string | null;
  name: string;
}

export function listPeople(): Promise<People> {
  return invoke<People>("list_people");
}

export function setRole(member: string, role: Role | null, canShare: boolean): Promise<void> {
  return invoke("set_role", { member, role, canShare });
}

export function assignDevice(device: string, member: string | null): Promise<void> {
  return invoke("assign_device", { device, member });
}

/** Sends a person away from the active library (D98). They stay in the catalog. */
export function withdrawPerson(member: string): Promise<void> {
  return invoke("withdraw_person", { member });
}

/** Takes the mark off a person sent away: it gives nothing back by itself (D98). */
export function restorePerson(member: string): Promise<void> {
  return invoke("restore_person", { member });
}

/** A lost device: out of every library, wherever it is. Returns how many. */
export function removeDevice(device: string): Promise<number> {
  return invoke<number>("remove_device", { device });
}

/** Removes this device's earlier installs from every library; returns how many. */
export function removeEarlierInstalls(library: string): Promise<number> {
  return invoke<number>("remove_earlier_installs", { library });
}

/** Who a newly paired device may belong to, in the libraries it receives (D73). */
export interface PairingOwners {
  /** Ids of the libraries where this device says whose it is. */
  libraries: string[];
  people: PairingCandidate[];
}

export interface PairingCandidate {
  id: string;
  name: string;
  /** The person this device belongs to. */
  me: boolean;
  /** Their role in each of those libraries where they have one. */
  roles: Record<string, Role>;
  devices: number;
}

export function pairingOwners(device: string): Promise<PairingOwners> {
  return invoke<PairingOwners>("pairing_owners", { device });
}

/** `libraries`: the ones the step showed (`PairingOwners.libraries`); no other is touched. */
export function giveAccess(
  device: string,
  person: PersonPick,
  roles: Record<string, Role>,
  canShare: boolean,
  libraries: string[],
): Promise<void> {
  return invoke("give_access", { device, person, roles, canShare, libraries });
}

/** The i18n key that explains a refusal by the roles, or null for other errors. */
export function permissionErrorKey(error: unknown): string | null {
  return isCommandError(error) && error.kind === "permission"
    ? `people.errors.${error.reason}`
    : null;
}

// --- Requests (D73), mirrors src-tauri/src/requests.rs --------------------------

export type RequestStatus = "pending" | "accepted" | "rejected" | "withdrawn";

export interface RequestSummary {
  action: "add_book" | "add_copy" | "delete_copy" | "delete_book" | "edit" | "other";
  title: string | null;
  authors: string[];
  year: number | null;
  publisher: string | null;
  format: Format | null;
  location: string | null;
}

export interface ChangeRequest {
  id: string;
  intent: string;
  deviceId: string;
  deviceName: string | null;
  person: string | null;
  /** Asked from this device or another device of the same person. */
  mine: boolean;
  /** Milliseconds since the Unix epoch. */
  at: number;
  note: string | null;
  status: RequestStatus;
  resolvedBy: string | null;
  resolvedAt: number | null;
  summary: RequestSummary;
  /** Accepting would still work. */
  applies: boolean;
  /** A new book: it can be corrected before accepting. */
  editable: boolean;
}

export function listRequests(): Promise<ChangeRequest[]> {
  return invoke<ChangeRequest[]>("list_requests");
}

export function acceptRequest(id: string): Promise<void> {
  return invoke("accept_request", { id });
}

export function rejectRequest(id: string): Promise<void> {
  return invoke("reject_request", { id });
}

export function withdrawRequest(id: string): Promise<void> {
  return invoke("withdraw_request", { id });
}

/** A book request as the add form fills it in, for «Edit and accept». */
export function requestedBook(id: string): Promise<NewBook> {
  return invoke<NewBook>("requested_book", { id });
}

export function acceptRequestEdited(id: string, book: NewBook): Promise<CreatedBook> {
  return invoke<CreatedBook>("accept_request_edited", { id, book });
}

/** This device's changes that became requests on recalculating the library. */
export function requestNotices(): Promise<ChangeRequest[]> {
  return invoke<ChangeRequest[]>("request_notices");
}

export function dismissRequestNotices(): Promise<void> {
  return invoke("dismiss_request_notices");
}

// --- Marks (D68), mirrors src-tauri/src/marks.rs ---------------------------------

export type MarkStatus = "pending" | "in_progress" | "finished" | "abandoned";
export const markStatuses: MarkStatus[] = ["pending", "in_progress", "finished", "abandoned"];

export interface Mark {
  status: MarkStatus | null;
  /** `YYYY-MM-DD`: when the status was set. */
  statusOn: string | null;
  /** From 1 to 5. */
  rating: number | null;
  favorite: boolean;
  wished: boolean;
}

export const emptyMark: Mark = {
  status: null,
  statusOn: null,
  rating: null,
  favorite: false,
  wished: false,
};

export interface PersonMark extends Mark {
  person: string;
}

export interface WorkMarks {
  mine: Mark | null;
  others: PersonMark[];
  /** This device belongs to a person, so it can mark. */
  canMark: boolean;
}

export function workMarks(workId: string): Promise<WorkMarks> {
  return invoke<WorkMarks>("work_marks", { workId });
}

export function setMark(workId: string, mark: Mark): Promise<void> {
  return invoke("set_mark", { workId, mark });
}

export interface SeriesSummary {
  id: string;
  name: string;
  ordered: boolean;
  /** Distinct works in the series. */
  works: number;
  /** Works in the series with no copies. */
  missing: number;
  /** Works you have a copy of, out of `total`, which counts the gaps of an ordered series. */
  have: number;
  total: number;
  /** The first covers in the series' order, at most three. */
  covers: { file: string; kind: Kind }[];
}

export interface SeriesEntry {
  entryId: string;
  workId: string;
  /** The work's first edition by title. */
  editionId: string | null;
  title: string;
  kind: Kind;
  cover: string | null;
  position: number | null;
  copies: number;
}

export interface SeriesDetail {
  id: string;
  name: string;
  ordered: boolean;
  entries: SeriesEntry[];
  have: number;
  total: number;
}

export function listSeries(): Promise<SeriesSummary[]> {
  return invoke<SeriesSummary[]>("list_series");
}

export function getSeries(id: string): Promise<SeriesDetail> {
  return invoke<SeriesDetail>("get_series", { id });
}

export function createSeries(name: string, ordered: boolean): Promise<string> {
  return invoke<string>("create_series", { name, ordered });
}

export function updateSeries(id: string, name: string, ordered: boolean): Promise<void> {
  return invoke("update_series", { id, name, ordered });
}

export function deleteSeries(id: string): Promise<void> {
  return invoke("delete_series", { id });
}

export function addToSeries(
  seriesId: string,
  workId: string,
  position: number | null = null,
): Promise<string> {
  return invoke<string>("add_to_series", { seriesId, workId, position });
}

export function removeFromSeries(seriesId: string, workId: string): Promise<void> {
  return invoke("remove_from_series", { seriesId, workId });
}

export interface WorkPosition {
  workId: string;
  /** Need not be consecutive: a gap is a volume not catalogued yet (D61). */
  position: number | null;
}

/** Sets the number of each work in an ordered series, in one operation. */
export function reorderSeries(seriesId: string, positions: WorkPosition[]): Promise<void> {
  return invoke("reorder_series", { seriesId, positions });
}

export function addRequirement(workId: string, requiredWorkId: string): Promise<void> {
  return invoke("add_requirement", { workId, requiredWorkId });
}

export function removeRequirement(workId: string, requiredWorkId: string): Promise<void> {
  return invoke("remove_requirement", { workId, requiredWorkId });
}

// --- Account (D77, D81, hito 7.1), mirrors src-tauri/src/account/mod.rs -------

export interface AccountUser {
  id: string;
  email: string;
  name: string;
  locale: string;
}

export interface AccountStatus {
  server: string;
  official: boolean;
  signedIn: boolean;
  user: AccountUser | null;
  emailHint: string | null;
  /** Something learnt in the background, shown once (D77). */
  notice: "device_removed" | "signed_out_password_changed" | null;
}

export interface ServerInfo {
  name: string;
  apiVersion: number;
  registration: boolean;
  signInWithGoogle: boolean;
  twoFactor: boolean;
  /** Only some addresses may make an account; which ones is never said (D100). */
  registrationRestricted: boolean;
}

/** Local only, no network: what the account screens show right away. */
export function accountStatus(): Promise<AccountStatus> {
  return invoke<AccountStatus>("account_status");
}

export function dismissAccountNotice(): Promise<void> {
  return invoke("dismiss_account_notice");
}

/** `null` switches back to the official server. */
export function setAccountServer(url: string | null): Promise<void> {
  return invoke("set_account_server", { url });
}

export function accountServerInfo(): Promise<ServerInfo> {
  return invoke<ServerInfo>("account_server_info");
}

export function registerAccount(
  email: string,
  password: string,
  name: string,
  locale: string,
): Promise<void> {
  return invoke("register_account", { email, password, name, locale });
}

/** The password is chosen here, with the code (D100): the one of whoever has the email. */
export function confirmAccountEmail(email: string, code: string, password: string): Promise<void> {
  return invoke("confirm_account_email", { email, code, password });
}

export function resendAccountCode(email: string): Promise<void> {
  return invoke("resend_account_code", { email });
}

/** `two_factor_required`: ask for the code and call again with `totpCode` set. */
export function signIn(
  email: string,
  password: string,
  totpCode: string | null = null,
): Promise<AccountUser> {
  return invoke<AccountUser>("sign_in", { email, password, totpCode });
}

/**
 * «Continuar con Google» (D77): asks the account's server for an attempt and opens the system browser at it. The
 * way back arrives by `kosmotheka://auth/google` and is read by `completeGoogleReturn`; nothing of it reaches here.
 */
export function startGoogle(locale: string): Promise<void> {
  return invoke("start_google", { locale });
}

/** «Cancel» while the browser is open. */
export function cancelGoogle(): Promise<void> {
  return invoke("cancel_google");
}

/**
 * The way back from Google, if one arrived: signs this device in, or throws why not (`two_factor_required` asks for
 * the code: `finishGoogle`). `null` when nothing was waiting or it was not this app's own attempt.
 */
export function completeGoogleReturn(): Promise<AccountUser | null> {
  return invoke<AccountUser | null>("complete_google_return");
}

/** The second factor's code, for the way back that was waiting for it. */
export function finishGoogle(totpCode: string): Promise<AccountUser> {
  return invoke<AccountUser>("finish_google", { totpCode });
}

export function signOut(): Promise<void> {
  return invoke("sign_out");
}

export function forgotPassword(email: string): Promise<void> {
  return invoke("forgot_password", { email });
}

/** `two_factor_required`: the email alone is not enough; ask for the code and call again with `totpCode` (D100). */
export function resetPassword(
  email: string,
  code: string,
  password: string,
  totpCode: string | null = null,
): Promise<void> {
  return invoke("reset_password", { email, code, password, totpCode });
}

// --- Account screen, second factor, devices and handing the device away (hito 7.1) ---

export interface AccountDetails {
  id: string;
  email: string;
  name: string;
  locale: string;
  hasPassword: boolean;
  createdAt: string;
  twoFactorEnabled: boolean;
  backupCodesLeft: number;
}

export function getAccount(): Promise<AccountDetails> {
  return invoke<AccountDetails>("get_account");
}

/** Each field is only sent when changed; the others keep their current value. */
export function updateAccount(fields: { name?: string; locale?: string }): Promise<AccountDetails> {
  return invoke<AccountDetails>("update_account", {
    name: fields.name ?? null,
    locale: fields.locale ?? null,
  });
}

export function changePassword(
  currentPassword: string,
  newPassword: string,
  totpCode: string | null = null,
): Promise<void> {
  return invoke("change_password", { currentPassword, newPassword, totpCode });
}

/**
 * What an account gives, besides its session, to do something that changes how it is entered (D100): the password,
 * or for one with none (it came in with Google) a code from its email; and a code of the second factor if it has it on.
 */
export interface ActionProof {
  password?: string;
  emailCode?: string;
  totpCode?: string;
}

/** Sends the account's own email the code that stands in for a password it does not have. */
export function requestActionCode(): Promise<void> {
  return invoke("request_action_code");
}

export function deleteAccount(proof: ActionProof): Promise<void> {
  return invoke("delete_account", { proof });
}

export interface AccountDevice {
  id: string;
  name: string;
  createdAt: string;
  lastSeenAt: string | null;
  signedIn: boolean;
  removed: boolean;
  /** `remove_account_device` just marked it removed; it has not reconnected yet. */
  canRestore: boolean;
  current: boolean;
}

export function accountDevices(): Promise<AccountDevice[]> {
  return invoke<AccountDevice[]>("account_devices");
}

export function removeAccountDevice(id: string): Promise<void> {
  return invoke("remove_account_device", { id });
}

/** Fails with `device_already_reset` once the device has reconnected: too late to undo. */
export function restoreAccountDevice(id: string): Promise<void> {
  return invoke("restore_account_device", { id });
}

export interface TwoFactorSetup {
  secret: string;
  otpauthUri: string;
}

export function twoFactorSetup(): Promise<TwoFactorSetup> {
  return invoke<TwoFactorSetup>("two_factor_setup");
}

/** Returns the ten backup codes, shown only this once. */
export function twoFactorEnable(code: string, proof: ActionProof): Promise<string[]> {
  return invoke<string[]>("two_factor_enable", { code, proof });
}

/** `password` only for an account that has one: without it (Google) the code of the second factor is the proof. */
export function twoFactorDisable(password: string | null, code: string): Promise<void> {
  return invoke("two_factor_disable", { password, code });
}

/** Returns a fresh set of ten backup codes; the old ones stop working. */
export function regenerateBackupCodes(password: string | null, code: string): Promise<string[]> {
  return invoke<string[]>("regenerate_backup_codes", { password, code });
}

/** Resets this device (D81): wipes its libraries and, if there was one, its session. */
export function giveDeviceAway(): Promise<void> {
  return invoke("give_device_away");
}

// --- The cloud (D84, hito 7.2) ---

type CloudErrorReason =
  | "not_found"
  | "schema_version"
  | "not_signed_in"
  | "unsigned_history"
  | "already_linked"
  | "no_cloud_device"
  | "access_withdrawn"
  | InvitationAppReason;

/** Account errors that come with a cloud call and have their own text under `cloud.errors`. */
const CLOUD_ACCOUNT_REASONS = ["link_rejected", "link_not_allowed", "events_unavailable"];

export type CloudOutcome =
  | "synced"
  | "downloaded"
  | "library_conflict"
  | "not_linked"
  | "missing_in_cloud"
  | "other_server"
  | "deleted_here"
  | "removed"
  | "access_withdrawn";

/** What a cloud sync did to one library. */
export interface CloudLibraryReport {
  library: string;
  cloudLibrary: string | null;
  outcome: CloudOutcome;
  received: number;
  sent: number;
  /** Operations dropped: from another library, invalid, or refused by the cloud. */
  rejected: number;
  /** With outcome "failed": the code of the error, when it has one. */
  error?: string;
  /** With outcome "removed": the library's name, which is gone by the time it is told. */
  libraryName?: string;
  /** The account was let into this library as this person, the first time this device heard of it (D76). */
  joined?: { person: string; role: Role | null };
  /** With outcome "downloaded": who this device is in the library by the end of the session, if anyone. */
  me?: { person: string; role: Role | null };
}

export interface LibraryCloudStatus {
  library: string;
  cloudLibrary: string | null;
  cloudServer: string | null;
  /** Linked to a cloud library on the account's current server. */
  linkedHere: boolean;
  signedIn: boolean;
  mayShare: boolean;
  /** Its log has operations without a signature: «Duplicate as new» instead (D58). */
  unsignedHistory: boolean;
  canUpload: boolean;
  /** This device administers it: only then it uploads it or changes the cloud's power. */
  administers: boolean;
  /** The cloud may link new devices of linked accounts to their person (D76). */
  cloudPower: boolean;
}

/** Local only, no network. */
export function cloudStatus(): Promise<LibraryCloudStatus[]> {
  return invoke<LibraryCloudStatus[]>("cloud_status");
}

/** Takes a library to the cloud of this account. */
export function uploadToCloud(libraryId: string, autolink: boolean): Promise<CloudLibraryReport> {
  return invoke<CloudLibraryReport>("upload_to_cloud", { libraryId, autolink });
}

/** Lets the cloud link new devices of this library (D76), or stops it. Only who administers it. */
export function setCloudPower(libraryId: string, on: boolean): Promise<void> {
  return invoke("set_cloud_power", { libraryId, on });
}

/** A library where the account is one person and this device belongs to another (D76). */
export interface OtherOwner {
  library: string;
  libraryName: string;
  /** The person this device belongs to there. */
  person: string;
  role: Role | null;
}

/** Questions about «this device is somebody else's» still open for the account signed in. Local only. */
export function cloudOtherOwners(): Promise<OtherOwner[]> {
  return invoke<OtherOwner[]>("cloud_other_owners");
}

/** «Leave it as it is»: not asked again for this account and this person. */
export function answerOtherOwner(libraryId: string): Promise<void> {
  return invoke("answer_other_owner", { libraryId });
}

/** One report per library of the account. */
export function syncCloud(): Promise<CloudLibraryReport[]> {
  return invoke<CloudLibraryReport[]>("sync_cloud");
}

/** The live connection to the server's hub (D75). */
export interface CloudConnection {
  live: boolean;
  /** `false` when the server has no hub or nobody is signed in: nothing to wait for. */
  available: boolean;
}

/** For the first render; `cloud-connection` events keep it current. */
export async function cloudConnection(): Promise<CloudConnection> {
  const connection = await invoke<CloudConnection | null>("cloud_connection");
  return connection ?? { live: false, available: false };
}

/** The app is back in the foreground: retry the live connection now. */
export function cloudEventsWake(): Promise<void> {
  return invoke("cloud_events_wake");
}

/** The cloud commands also fail with the account's errors: no connection is one of them. */
export function isCloudOffline(error: unknown): boolean {
  return isCommandError(error) && error.kind === "account" && error.reason === "server_unavailable";
}

/** Translation key for a failed cloud command. */
export function cloudErrorKey(error: unknown): string {
  if (isCommandError(error) && error.kind === "cloud") return `cloud.errors.${error.reason}`;
  if (
    isCommandError(error) &&
    error.kind === "account" &&
    CLOUD_ACCOUNT_REASONS.includes(error.reason)
  )
    return `cloud.errors.${error.reason}`;
  if (isCloudOffline(error)) return "cloud.errors.offline";
  return "cloud.errors.unknown";
}

// --- Invitations (D43, milestone 7.6.2) ---

/** The app's own reasons for the invitation commands, before or instead of asking the server. */
type InvitationAppReason =
  | "not_in_cloud"
  | "no_cloud_power"
  | "server_without_invitations"
  | "invalid_email"
  | "invalid_name"
  | "link_needs_new_person"
  | "person_not_invitable"
  | "invitation_bad_link"
  | "invitation_other_server"
  | "invitation_not_found"
  | "invitation_no_pending"
  | "invitation_other_account";

export type InvitationKind = "email" | "link";

type InvitationState = "pending" | "accepted" | "revoked" | "expired" | "unknown";

/**
 * Why a library cannot invite now.
 */
type InviteBlocker =
  "not_signed_in" | "not_in_cloud" | "not_admin" | "no_cloud_power" | "server_without_invitations";

export interface InviteStatus {
  canInvite: boolean;
  blocker: InviteBlocker | null;
}

/**
 * Who an invitation is for. A new person is always created as a reader: there is no role to give, and Rust
 * refuses a request that names one.
 */
type InvitePerson = { type: "new"; name: string } | { type: "existing"; member: string };

export interface InviteRequest {
  person: InvitePerson;
  kind: InvitationKind;
  /** Only for `email`: sent as typed, never kept. */
  email?: string | null;
}

interface CreatedInvitation {
  id: string;
  kind: InvitationKind;
  memberId: string;
  expiresAt: string;
  /** Only for `link`, and only here: show it once, do not keep it. */
  link: string | null;
}

/**
 * What inviting did. If `failed` is set after `personCreated`, the person is in the library anyway and the
 * invitation can be asked again for them (by email).
 */
export interface InviteResult {
  member: string;
  personCreated: boolean;
  invitation: CreatedInvitation | null;
  failed: string | null;
}

export interface InvitationItem {
  id: string;
  kind: InvitationKind;
  /** The person it is for: cross it with the list of people. */
  memberId: string;
  state: InvitationState;
  createdAt: string;
  expiresAt: string;
  acceptedAt: string | null;
}

/**
 * A link that was opened: what it says, and whether the account that accepts it is already signed in.
 */
export interface OpenedInvitation {
  libraryName: string;
  kind: InvitationKind;
  expiresAt: string;
  signedIn: boolean;
}

export interface AcceptedInvitation {
  cloudLibrary: string;
  /** The library on this device, if the session after accepting brought it down. */
  library: string | null;
  libraryName: string | null;
  reports: CloudLibraryReport[];
  /** Why the session after accepting failed. The invitation is used anyway; the next sync brings the library. */
  syncError: string | null;
}

/** The `invitation-accepted` event: somebody accepted an invitation to a library of this device. No personal data. */
export interface InvitationAcceptedNotice {
  library: string;
  libraryName: string;
}

/**
 * Can this library invite now, and if not, why. Needs no session.
 */
export function inviteStatus(libraryId: string): Promise<InviteStatus> {
  return invoke<InviteStatus>("invite_status", { libraryId });
}

export function inviteToLibrary(libraryId: string, request: InviteRequest): Promise<InviteResult> {
  return invoke<InviteResult>("invite_to_library", { libraryId, request });
}

export function listInvitations(libraryId: string): Promise<InvitationItem[]> {
  return invoke<InvitationItem[]>("list_invitations", { libraryId });
}

export function revokeInvitation(libraryId: string, invitationId: string): Promise<void> {
  return invoke("revoke_invitation", { libraryId, invitationId });
}

/**
 * Opens a pasted (or handed-over) invitation link, with no session; it waits for the account that accepts.
 */
export function openInvitation(link: string): Promise<OpenedInvitation> {
  return invoke<OpenedInvitation>("open_invitation", { link });
}

/**
 * Looks at the link the system handed to the app (a tap on `https://kosmotheka.app/i/…`), if one is waiting.
 * Same road as a pasted link, and no more than looking: the link itself never comes to the screen, so it cannot
 * end up in its state, its history or a message. `null` when nothing was waiting; a bad link throws as
 * `openInvitation` does.
 */
export function openArrivedLink(): Promise<OpenedInvitation | null> {
  return invoke<OpenedInvitation | null>("open_arrived_link");
}

/**
 * What waits for a session after a link was opened: nothing, the link, or a link that expired while the person
 * signed in. The note of an expired one stays until `discardPendingInvitation`.
 */
export type PendingInvitation =
  | { state: "nothing" }
  | ({ state: "waiting" } & OpenedInvitation)
  | { state: "expired"; libraryName: string };

export function pendingInvitation(): Promise<PendingInvitation> {
  return invoke<PendingInvitation>("pending_invitation");
}

export function discardPendingInvitation(): Promise<void> {
  return invoke("discard_pending_invitation");
}

/**
 * Accepts the waiting link with the signed-in account and syncs, so the library comes down.
 */
export function acceptInvitation(): Promise<AcceptedInvitation> {
  return invoke<AcceptedInvitation>("accept_invitation");
}

/** The server's own codes for invitations, which have their own text under `invitations.errors`. */
const INVITATION_REASONS = [
  "invitations_unavailable",
  "invitation_forbidden",
  "invitation_library_not_ready",
  "invitation_person_unsuitable",
  "invitation_expired",
  "invitation_used",
  "invitation_revoked",
  "invitation_not_for_this_account",
  "invitation_account_already_in_library",
  "too_many_requests",
  "not_signed_in",
  "not_in_cloud",
  "no_cloud_power",
  "server_without_invitations",
  "invalid_email",
  "invalid_name",
  "link_needs_new_person",
  "person_not_invitable",
  "invitation_bad_link",
  "invitation_other_server",
  "invitation_not_found",
  "invitation_no_pending",
  "invitation_other_account",
  "server_unavailable",
];

/**
 * Translation key for the code of a failed invitation command, or of `InviteResult.failed`.
 */
export function invitationReasonKey(reason: string): string {
  return INVITATION_REASONS.includes(reason)
    ? `invitations.errors.${reason}`
    : "invitations.errors.unknown";
}

export function invitationErrorKey(error: unknown): string {
  if (isCommandError(error)) {
    if (error.kind === "permission" && error.reason === "not_allowed")
      return "invitations.errors.invitation_forbidden";
    if (error.kind === "account" || error.kind === "cloud")
      return invitationReasonKey(error.reason);
  }
  return "invitations.errors.unknown";
}

/** Keeps a cover of one's own (a photo or an image, in base64), reduced like any other (D82): its file name. */
export function storeOwnCover(photo: string): Promise<string> {
  return invoke<string>("store_own_cover", { photo });
}
