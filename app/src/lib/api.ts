import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { join } from "@tauri-apps/api/path";

// These types mirror the Rust structs in crates/domain and src-tauri/src/catalog.rs.

export const formats = [
  "hardcover",
  "paperback",
  "pocket",
  "ebook",
  "audiobook",
  "other",
] as const;
export type Format = (typeof formats)[number];

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
  googleBooksId: string | null;
  openLibraryId: string | null;
}

export interface CopyInput {
  owner: string | null;
  format: Format;
  quantity: number;
  location: string | null;
  acquiredOn: string | null;
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
}

export type FieldErrorCode =
  | "required"
  | "out_of_range"
  | "invalid_date"
  | "isbn_length"
  | "isbn_character"
  | "isbn_checksum"
  | "isbn_in_use"
  | "invalid_cover_url"
  | "invalid_format"
  | "name_in_use";

export interface FieldError {
  field: string;
  code: FieldErrorCode;
}

export type CommandError =
  | { kind: "validation"; errors: FieldError[] }
  | { kind: "backup"; reason: string }
  | { kind: "sync"; reason: string }
  | { kind: "permission"; reason: "not_allowed" | "last_admin" | "request_resolved" }
  | { kind: "internal"; message: string };

export function isCommandError(value: unknown): value is CommandError {
  return typeof value === "object" && value !== null && "kind" in value;
}

export function createBook(book: NewBook): Promise<CreatedBook> {
  return invoke<CreatedBook>("create_book", { book });
}

export interface BookFilter {
  query?: string;
  format?: Format;
  /** Owner name; matches duplicates created on other devices too. */
  owner?: string;
  location?: string;
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
}

export interface BookDetail {
  editionId: string;
  workId: string;
  title: string;
  subtitle: string | null;
  authors: string[];
  publisher: string | null;
  publishedYear: number | null;
  language: string | null;
  isbn13: string | null;
  cover: string | null;
  googleBooksId: string | null;
  openLibraryId: string | null;
  copies: CopyDetail[];
}

export function getBook(editionId: string): Promise<BookDetail> {
  return invoke<BookDetail>("get_book", { editionId });
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

export type Source = "google_books" | "open_library";

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
  warnings: { source: Source; kind: "network" | "quota" | "unexpected" }[];
  existingEditionId: string | null;
}

export function lookupIsbn(isbn: string): Promise<LookupResult> {
  return invoke<LookupResult>("lookup_isbn", { isbn });
}

export interface Settings {
  googleBooksApiKey: string | null;
}

export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
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
  googleBooks: (id: string) =>
    `https://books.google.com/books?id=${encodeURIComponent(id)}`,
  openLibrary: (id: string) =>
    `https://openlibrary.org/books/${encodeURIComponent(id)}`,
};

export interface BackupSummary {
  libraries: number;
  books: number;
  copies: number;
  covers: number;
}

/** What a restore did, library by library. */
export interface RestoreSummary {
  restored: { id: string; name: string; books: number; copies: number }[];
  /** Libraries this device already had with content; left as they were. */
  skipped: string[];
  covers: number;
}

/** Backs up the active library, or `library` if given. */
export function exportBackup(path: string, library?: string): Promise<BackupSummary> {
  return invoke<BackupSummary>("export_backup", { path, library });
}

/** Backs up every library on this device in one file. */
export function exportDeviceBackup(path: string): Promise<BackupSummary> {
  return invoke<BackupSummary>("export_device_backup", { path });
}

export function restoreBackup(path: string): Promise<RestoreSummary> {
  return invoke<RestoreSummary>("restore_backup", { path });
}

export function exportCsv(path: string): Promise<number> {
  return invoke<number>("export_csv", { path });
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
): Promise<{ code: string; expiresInSeconds: number }> {
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

export interface Wish {
  id: string;
  title: string;
  author: string | null;
  isbn13: string | null;
  note: string | null;
  coverUrl: string | null;
  createdAt: string;
}

export interface WishInput {
  title: string;
  author?: string | null;
  isbn?: string | null;
  note?: string | null;
  coverUrl?: string | null;
}

export interface IsbnCheck {
  isbn13: string;
  books: BookSummary[];
  wishes: Wish[];
}

export function addWish(wish: WishInput): Promise<string> {
  return invoke<string>("add_wish", { wish });
}

export function listWishes(): Promise<Wish[]> {
  return invoke<Wish[]>("list_wishes");
}

export function deleteWish(id: string): Promise<void> {
  return invoke("delete_wish", { id });
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

// --- Libraries -----------------------------------------------------------------

export interface LibrarySummary {
  id: string;
  /** Empty for a library never named: show `nav.myLibrary` instead. */
  name: string;
  works: number;
  active: boolean;
  created: { device: string | null; at: number } | null;
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

/** Removes it from this device only. Returns the library now active. */
export function leaveLibrary(id: string): Promise<string> {
  return invoke<string>("leave_library", { id });
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
}

export interface Person {
  id: string;
  name: string;
  role: Role | null;
  canShare: boolean;
  devices: PersonDevice[];
  /** Other libraries on this device where the same person is. */
  elsewhere: string[];
}

export interface People {
  hasRoles: boolean;
  myRole: Role | null;
  people: Person[];
  /** Devices that belong to nobody: they count as readers. */
  unassigned: PersonDevice[];
  /** Only one person administers the library (D58). */
  loneAdmin: boolean;
}

/** Someone in other libraries of this device but not in the active one. */
export interface KnownPerson {
  id: string;
  name: string;
  libraries: string[];
}

/** A person picked on screen: an existing one (`id`) or somebody new (`name`). */
export interface PersonPick {
  id: string | null;
  name: string;
}

export interface DeviceOwner {
  deviceId: string;
  /** `null`: nobody's, so it counts as a reader. */
  person: PersonPick | null;
  role: Role | null;
}

export function listPeople(): Promise<People> {
  return invoke<People>("list_people");
}

export function knownPeople(): Promise<KnownPerson[]> {
  return invoke<KnownPerson[]>("known_people");
}

export function startRoles(name: string, owners: DeviceOwner[]): Promise<string> {
  return invoke<string>("start_roles", { name, owners });
}

export function setRole(member: string, role: Role | null, canShare: boolean): Promise<void> {
  return invoke("set_role", { member, role, canShare });
}

export function assignDevice(device: string, member: string | null): Promise<void> {
  return invoke("assign_device", { device, member });
}

/** A lost device: out of every library, wherever it is. Returns how many. */
export function removeDevice(device: string): Promise<number> {
  return invoke<number>("remove_device", { device });
}

export function giveAccess(
  device: string,
  person: PersonPick,
  roles: Record<string, Role>,
  canShare: boolean,
): Promise<void> {
  return invoke("give_access", { device, person, roles, canShare });
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
  action: "add_book" | "add_copy" | "delete_copy" | "delete_book" | "edit" | "add_wish" | "other";
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
