// Preview mode for design work: open the Vite dev server with `?mock` in a
// normal browser and the Rust commands answer with sample data. Only loaded
// in development (see main.tsx); never part of a build.
import { mockIPC } from "@tauri-apps/api/mocks";

import type {
  AccountDetails,
  AccountDevice,
  AccountStatus,
  AccountUser,
  BookFilter,
  BookSummary,
  Candidate,
  ChangeRequest,
  Holding,
  Kind,
  LanStatus,
  LibrarySummary,
  Mark,
  People,
  PersonMark,
  RelatedWork,
  RequestSummary,
  ServerInfo,
  SeriesDetail,
  SeriesEntry,
  Settings,
  TwoFactorSetup,
  WorkSeries,
} from "@/lib/api";

// Mutable so the Providers screen can save keys and toggles across calls,
// preserved only for the life of this dev-server tab.
let mockSettings: Settings = { apiKeys: {}, disabledSources: [] };

// The biometric plugin itself cannot be mocked through IPC, so the switch always
// shows as unavailable here; `?lock` renders the lock screen directly instead.
let mockAppLock = false;
let mockWelcomeSeen = 0;
let mockAppLockAfter = 60;

const HOME = "01997c7e-0000-7000-8000-00000000a0e1";
const CLUB = "01997c7e-0000-7000-8000-00000000c1b0";

const libraries: LibrarySummary[] = [
  {
    id: HOME,
    name: "Casa",
    works: 7,
    active: true,
    created: null,
    // `?noshare`: this device may not share Casa.
    canShare: !new URLSearchParams(window.location.search).has("noshare"),
  },
  {
    id: CLUB,
    name: "Club de rol",
    works: 3,
    active: false,
    created: { device: "Móvil de Laura", at: Date.parse("2026-09-25T18:00:00Z") },
    canShare: true,
  },
];

// Cloud (D84, hito 7.2), with `?account`: `?cloud` adds a library with unsigned history and one
// this device may not share, and starts Casa already in the cloud. `?cloud=offline`,
// `conflict`, `missing` and `deleted` make the sync report that instead of «synced».
const RECIPES = "01997c7e-0000-7000-8000-00000000ec1e";
const GAMES = "01997c7e-0000-7000-8000-00000000ba5e";
const cloudMode = new URLSearchParams(window.location.search).get("cloud");
if (cloudMode !== null) {
  libraries.push(
    {
      id: RECIPES,
      name: "Recetas de la abuela",
      works: 31,
      active: false,
      created: null,
      canShare: true,
    },
    {
      id: GAMES,
      name: "Videojuegos de Hugo",
      works: 12,
      active: false,
      created: null,
      canShare: false,
    },
  );
}
const cloudLinked = new Set<string>(cloudMode !== null ? [HOME] : []);

// Invitations (D43, 7.6.2): the shape of the answers, with no server behind. The token is not a real one.
const MOCK_TOKEN = "A".repeat(43);
interface MockInvitation {
  id: string;
  kind: string;
  memberId: string;
  state: string;
  createdAt: string;
  expiresAt: string;
  acceptedAt: string | null;
}
// `?invited` seeds People with an unused invitation (Laura) and an accepted one (Hugo).
const mockInvitations: MockInvitation[] = new URLSearchParams(window.location.search).has("invited")
  ? [
      {
        id: "invitation-laura",
        kind: "email",
        memberId: "laura",
        state: "pending",
        createdAt: new Date().toISOString(),
        expiresAt: new Date(Date.now() + 20 * 3600 * 1000).toISOString(),
        acceptedAt: null,
      },
      {
        id: "invitation-hugo",
        kind: "link",
        memberId: "hugo",
        state: "accepted",
        createdAt: new Date(Date.now() - 3600 * 1000).toISOString(),
        expiresAt: new Date(Date.now() + 23 * 3600 * 1000).toISOString(),
        acceptedAt: new Date().toISOString(),
      },
    ]
  : [];
let mockPendingInvitation: {
  libraryName: string;
  kind: string;
  expiresAt: string;
  signedIn: boolean;
} | null = null;

const work = (
  n: number,
  kind: Kind,
  title: string,
  authors: string[],
  year: number,
  formats: BookSummary["formats"],
  owners: string[],
  publisher: string | null = null,
): BookSummary => ({
  libraryId: HOME,
  editionId: `01997c7e-0000-7000-8000-${String(n).padStart(12, "0")}`,
  kind,
  title,
  subtitle: null,
  authors,
  publisher,
  publishedYear: year,
  isbn13: null,
  cover: null,
  copyCount: formats.length,
  formats,
  owners,
  myStatus: null,
  myFavorite: false,
  myWished: false,
  wishedBy: [],
  tags: [],
});

const book = (
  n: number,
  title: string,
  authors: string[],
  year: number,
  formats: BookSummary["formats"],
  owners: string[],
): BookSummary => work(n, "book", title, authors, year, formats, owners, "Debolsillo");

const books: BookSummary[] = [
  book(1, "Dune", ["Frank Herbert"], 1965, ["paperback", "ebook"], ["Tersoal", "Laura"]),
  book(2, "El mesías de Dune", ["Frank Herbert"], 1969, ["paperback"], ["Tersoal"]),
  book(3, "Fundación", ["Isaac Asimov"], 1951, ["pocket"], ["Tersoal"]),
  book(4, "La mano izquierda de la oscuridad", ["Ursula K. Le Guin"], 1969, ["ebook"], ["Laura"]),
  book(5, "El nombre del viento", ["Patrick Rothfuss"], 2007, ["hardcover"], ["Tersoal"]),
  book(
    6,
    "Crónicas de la Dragonlance",
    ["Margaret Weis", "Tracy Hickman"],
    1987,
    ["paperback"],
    ["Hugo"],
  ),
  book(7, "Vampiro: la mascarada", ["Mark Rein-Hagen"], 1991, ["hardcover", "other"], ["Tersoal"]),
  // Non-book kinds (D60), so the catalog can be seen mixed.
  work(8, "film", "Blade Runner 2049", ["Denis Villeneuve"], 2017, ["bluray"], ["Tersoal"]),
  work(9, "boardgame", "Catán", [], 1995, ["box"], ["Hugo", "Laura"], "Devir"),
  work(
    10,
    "videogame",
    "Zelda: Tears of the Kingdom",
    [],
    2023,
    ["physical", "digital"],
    ["Hugo", "Tersoal"],
  ),
  work(11, "tv_series", "Twin Peaks", [], 1990, ["dvd", "digital"], ["Tersoal", "Laura"]),
  work(12, "music", "Abbey Road", ["The Beatles"], 1969, ["vinyl"], ["Tersoal"]),
  work(13, "other", "Playmobil Barco pirata 5135", [], 2006, ["other"], ["Laura"]),
  // "Me falta" (D61): catalogued, no copies yet. Who else wishes each one is set below.
  work(14, "book", "Dune: El emperador-dios", ["Frank Herbert"], 1981, [], []),
  work(15, "book", "Hijos de Dune", ["Frank Herbert"], 1976, [], []),
  work(16, "boardgame", "Catán: Navegantes", [], 2015, [], [], "Devir"),
  work(17, "film", "Blade Runner", ["Ridley Scott"], 1982, [], []),
  // Series and «necesita» (D61): Casa Capitular fills the Dune saga's position 6, leaving 5
  // open as a gap; Imperium's expansion needs its base, which nobody has.
  work(18, "book", "Casa Capitular: Dune", ["Frank Herbert"], 1985, ["hardcover"], ["Tersoal"]),
  work(19, "boardgame", "Dune: Imperium", [], 2020, [], [], "Dire Wolf"),
  work(
    20,
    "boardgame",
    "Dune: Imperium – Ascenso de Ix",
    [],
    2022,
    ["box"],
    ["Tersoal"],
    "Dire Wolf",
  ),
];

/** A fixture book by its index: the fixtures are fixed, so a miss is a typo here. */
function bookAt(index: number): BookSummary {
  const book = books[index];
  if (!book) throw new Error(`mock: no book at ${index}`);
  return book;
}
// Fundación keeps a pocket copy and an ISBN, for the change-of-kind screenshots
// (book -> comic): the ISBN becomes a barcode and "Bolsillo" needs a new format.
books[2] = { ...bookAt(2), isbn13: "9788497596824" };
// Dune shows a filled ISBN row on its own work page.
books[0] = { ...bookAt(0), isbn13: "9788497596848" };

// Who else wishes a missing work (D61); this device's own wish is a mark, set below
// so toggling "Lo deseo" on the work page stays consistent with the catalog rows.
const otherWishersByEdition: Record<string, string[]> = {
  [bookAt(13).editionId]: ["Hugo"],
  [bookAt(15).editionId]: ["Laura"],
  [bookAt(16).editionId]: ["Laura"],
};
function wishedByFor(editionId: string): string[] {
  const mine = myMarks.get(`work-${editionId}`)?.wished ?? false;
  const others = otherWishersByEdition[editionId] ?? [];
  return mine ? ["me", ...others] : others;
}

// Series and «necesita» (D61): an ordered Dune saga with a gap at 5, and an unordered
// Catán line whose expansion needs the base (both already owned, unlike Imperium's).
const DUNE_SERIES = "series-1";
const CATAN_SERIES = "series-2";
const workIdOf = (editionId: string) => `work-${editionId}`;
const relatedWork = (b: BookSummary): RelatedWork => ({
  workId: workIdOf(b.editionId),
  editionId: b.editionId,
  title: b.title,
  copies: b.copyCount,
});

const seriesOfEdition: Record<string, WorkSeries[]> = {
  [bookAt(0).editionId]: [{ seriesId: DUNE_SERIES, name: "Dune", ordered: true, position: 1 }],
  [bookAt(1).editionId]: [{ seriesId: DUNE_SERIES, name: "Dune", ordered: true, position: 2 }],
  [bookAt(14).editionId]: [{ seriesId: DUNE_SERIES, name: "Dune", ordered: true, position: 3 }],
  [bookAt(13).editionId]: [{ seriesId: DUNE_SERIES, name: "Dune", ordered: true, position: 4 }],
  [bookAt(17).editionId]: [{ seriesId: DUNE_SERIES, name: "Dune", ordered: true, position: 6 }],
  [bookAt(8).editionId]: [
    { seriesId: CATAN_SERIES, name: "Catán", ordered: false, position: null },
  ],
  [bookAt(15).editionId]: [
    { seriesId: CATAN_SERIES, name: "Catán", ordered: false, position: null },
  ],
};

const requiresOfEdition: Record<string, RelatedWork[]> = {
  // Catán: Navegantes needs Catán (owned): no warning, just the link.
  [bookAt(15).editionId]: [relatedWork(bookAt(8))],
  // Imperium's expansion needs the base, which nobody owns: the amber notice shows.
  [bookAt(19).editionId]: [relatedWork(bookAt(18))],
};
const requiredByOfEdition: Record<string, RelatedWork[]> = {
  [bookAt(8).editionId]: [relatedWork(bookAt(15))],
  [bookAt(18).editionId]: [relatedWork(bookAt(19))],
};

// Genres and tags (D70, 6.7): the closed list per kind, mirroring
// crates/domain/src/genre.rs, and a small mutable tag store so rename/merge/delete
// behave plausibly across the Tags page, the form and the catalog filters.
const GENRES_BY_KIND: Record<Kind, string[]> = {
  book: [
    "novel",
    "science_fiction",
    "fantasy",
    "horror",
    "mystery",
    "historical",
    "romance",
    "adventure",
    "poetry",
    "theatre",
    "essay",
    "biography",
    "popular_science",
    "children",
    "young_adult",
    "roleplaying",
    "cooking",
    "art",
    "travel",
    "others",
  ],
  comic: [
    "superheroes",
    "manga",
    "science_fiction",
    "fantasy",
    "horror",
    "humor",
    "adventure",
    "graphic_novel",
    "crime",
    "historical",
    "children",
    "others",
  ],
  boardgame: [
    "strategy",
    "family",
    "party",
    "cooperative",
    "abstract",
    "card_game",
    "tile_placement",
    "worker_placement",
    "deck_building",
    "deduction",
    "wargame",
    "children",
    "roleplaying",
    "others",
  ],
  film: [
    "action",
    "adventure",
    "animation",
    "science_fiction",
    "comedy",
    "documentary",
    "drama",
    "fantasy",
    "musical",
    "thriller",
    "horror",
    "romance",
    "war",
    "western",
    "crime",
    "children",
    "others",
  ],
  tv_series: [
    "action",
    "adventure",
    "animation",
    "science_fiction",
    "comedy",
    "documentary",
    "drama",
    "fantasy",
    "musical",
    "thriller",
    "horror",
    "romance",
    "war",
    "western",
    "crime",
    "children",
    "others",
  ],
  videogame: [
    "action",
    "adventure",
    "roleplaying",
    "strategy",
    "platformer",
    "puzzle",
    "racing",
    "sports",
    "fighting",
    "shooter",
    "simulation",
    "horror",
    "party",
    "others",
  ],
  music: [
    "rock",
    "pop",
    "jazz",
    "classical",
    "electronic",
    "hip_hop",
    "folk",
    "flamenco",
    "blues",
    "soul",
    "metal",
    "soundtrack",
    "children",
    "others",
  ],
  other: [],
};

const genresOfEdition: Record<string, string[]> = {
  [bookAt(0).editionId]: ["science_fiction"], // Dune (book)
  [bookAt(1).editionId]: ["science_fiction"], // El mesías de Dune
  [bookAt(2).editionId]: ["science_fiction"], // Fundación
  [bookAt(6).editionId]: ["roleplaying"], // Vampiro: la mascarada
  [bookAt(7).editionId]: ["science_fiction"], // Blade Runner 2049 (film)
  [bookAt(8).editionId]: ["strategy", "family"], // Catán (boardgame)
  [bookAt(13).editionId]: ["science_fiction"], // Dune: El emperador-dios
  [bookAt(14).editionId]: ["science_fiction"], // Hijos de Dune
  [bookAt(15).editionId]: ["strategy", "family"], // Catán: Navegantes
  [bookAt(16).editionId]: ["science_fiction"], // Blade Runner (film)
  [bookAt(17).editionId]: ["science_fiction"], // Casa Capitular: Dune
};

interface MockTag {
  id: string;
  name: string;
}

let mockTags: MockTag[] = [
  { id: "tag-1", name: "clásicos" },
  { id: "tag-2", name: "para Hugo" },
  { id: "tag-3", name: "regalos" },
];

const workTagIds: Record<string, string[]> = {
  [bookAt(0).editionId]: ["tag-1"], // Dune
  [bookAt(7).editionId]: ["tag-1"], // Blade Runner 2049
  [bookAt(16).editionId]: ["tag-1"], // Blade Runner
  [bookAt(9).editionId]: ["tag-2"], // Zelda: Tears of the Kingdom
  [bookAt(10).editionId]: ["tag-2"], // Twin Peaks
  [bookAt(4).editionId]: ["tag-3"], // El nombre del viento
};

function tagWorksCount(id: string): number {
  return Object.values(workTagIds).filter((ids) => ids.includes(id)).length;
}

function tagNamesOf(editionId: string): string[] {
  const ids = workTagIds[editionId] ?? [];
  return ids
    .map((id) => mockTags.find((t) => t.id === id)?.name)
    .filter((n): n is string => Boolean(n))
    .sort((a, b) => a.localeCompare(b));
}

function findOrCreateTag(name: string): MockTag {
  const trimmed = name.trim();
  const existing = mockTags.find((t) => t.name.toLowerCase() === trimmed.toLowerCase());
  if (existing) return existing;
  const created = { id: `tag-${mockTags.length + 1}-${Date.now()}`, name: trimmed };
  mockTags.push(created);
  return created;
}

function seriesEntryOf(b: BookSummary, entryId: string, position: number | null): SeriesEntry {
  return {
    entryId,
    workId: workIdOf(b.editionId),
    editionId: b.editionId,
    title: b.title,
    kind: b.kind,
    cover: b.cover,
    position,
    copies: b.copyCount,
  };
}

const seriesById: Record<string, SeriesDetail> = {
  [DUNE_SERIES]: {
    id: DUNE_SERIES,
    name: "Dune",
    ordered: true,
    entries: [
      seriesEntryOf(bookAt(0), "d1", 1),
      seriesEntryOf(bookAt(1), "d2", 2),
      seriesEntryOf(bookAt(14), "d3", 3),
      seriesEntryOf(bookAt(13), "d4", 4),
      // Position 5 is left out on purpose: that is the gap.
      seriesEntryOf(bookAt(17), "d6", 6),
    ],
  },
  [CATAN_SERIES]: {
    id: CATAN_SERIES,
    name: "Catán",
    ordered: false,
    entries: [seriesEntryOf(bookAt(8), "c1", null), seriesEntryOf(bookAt(15), "c2", null)],
  },
};

const lan: LanStatus = {
  deviceId: "dev",
  deviceName: "Portátil",
  servingPort: null,
  addresses: [],
  pairingOpen: false,
  servers: [
    {
      deviceId: "phone",
      name: "Móvil de Laura",
      pairedAt: "2026-09-24T10:00:00Z",
      lastSyncAt: new Date(Date.now() - 2 * 60000).toISOString(),
    },
    {
      deviceId: "tablet",
      name: "Tablet Lenovo",
      pairedAt: "2026-09-24T10:00:00Z",
      lastSyncAt: null,
    },
  ],
  clients: [],
};

// `?noroles` shows a library without roles yet; `?reader` sees it as Hugo.
const params = new URLSearchParams(window.location.search);

// `?welcome`: every library reports no works, so the first-run screen shows (D77).
if (params.has("welcome")) {
  for (const library of libraries) library.works = 0;
}

// Account (D77, D81, hito 7.1): `?account` starts signed in as "Ana Ruiz"; `?notice=…`
// sets a background notice; `?offline` makes every account command fail as if the
// cloud were unreachable.
const offline = () => params.has("offline");
let mockAccount: AccountStatus = {
  server: "https://api.kosmotheka.app",
  official: true,
  signedIn: params.has("account"),
  user: params.has("account")
    ? { id: "ana", email: "ana.ruiz@correo.es", name: "Ana Ruiz", locale: "es" }
    : null,
  emailHint: params.has("account") ? "ana.ruiz@correo.es" : null,
  notice:
    params.get("notice") === "device_removed" ||
    params.get("notice") === "signed_out_password_changed"
      ? (params.get("notice") as AccountStatus["notice"])
      : null,
};

// `?twofa` starts the second factor already on, with 8 of the 10 backup codes
// left (one used); otherwise it starts off, as a fresh account would be.
let mockAccountDetails: AccountDetails = {
  id: "ana",
  email: "ana.ruiz@correo.es",
  name: "Ana Ruiz",
  locale: "es",
  hasPassword: true,
  createdAt: "2026-01-15T10:00:00Z",
  twoFactorEnabled: params.has("twofa"),
  backupCodesLeft: params.has("twofa") ? 8 : 10,
};

const ACCOUNT_BACKUP_CODES = [
  "7KQ4M-2XDH2",
  "P9TA3-WLC9F",
  "H2ME8-QZRK4",
  "V6NBK-4JSQ2",
  "D3XU7-FGYB1",
  "R8CW2-HPN53",
  "L5ZT9-MKE86",
  "B4QJ6-VRA72",
  "N7SD1-XWF94",
  "F2GH5-TLB38",
];

let mockDevices: AccountDevice[] = [
  {
    id: "ana-phone",
    name: "Móvil de Ana",
    createdAt: "2026-01-15T10:00:00Z",
    lastSeenAt: new Date().toISOString(),
    signedIn: true,
    removed: false,
    canRestore: false,
    current: true,
  },
  {
    id: "laptop",
    name: "Portátil",
    createdAt: "2026-01-16T09:00:00Z",
    lastSeenAt: new Date(Date.now() - 2 * 3600_000).toISOString(),
    signedIn: true,
    removed: false,
    canRestore: false,
    current: false,
  },
  {
    id: "kitchen-tablet",
    name: "Tablet de la cocina",
    createdAt: "2026-02-01T09:00:00Z",
    lastSeenAt: new Date(Date.now() - 3 * 24 * 3600_000).toISOString(),
    signedIn: true,
    removed: false,
    canRestore: false,
    current: false,
  },
];

const dev = (id: string, name: string, thisDevice = false) => ({
  id,
  name,
  thisDevice,
  lastDeviceOfLastAdmin: false,
  linkedByCloud: id === "pixel",
  linkedBy: null,
});
const people: People = {
  hasRoles: !params.has("noroles"),
  myRole: params.has("reader") ? "reader" : "admin",
  loneAdmin: true,
  people: [
    {
      id: "ana",
      name: "Ana",
      role: "admin",
      canShare: true,
      accessWithdrawn: false,
      lastAdmin: true,
      hasAccount: true,
      invitable: "none",
      devices: [dev("dev", "Portátil", !params.has("reader")), dev("pixel", "Pixel 8")],
      elsewhere: ["Club de rol"],
    },
    {
      id: "laura",
      name: "Laura",
      role: "editor",
      canShare: true,
      accessWithdrawn: false,
      lastAdmin: false,
      hasAccount: false,
      invitable: "email",
      devices: [dev("phone", "Móvil de Laura")],
      elsewhere: [],
    },
    {
      id: "hugo",
      name: "Hugo",
      role: "reader",
      canShare: false,
      accessWithdrawn: params.has("withdrawn"),
      lastAdmin: false,
      hasAccount: false,
      invitable: params.has("withdrawn") ? "none" : "email",
      devices: [
        dev("hugo-phone", "Móvil de Hugo", params.has("reader")),
        dev("tablet", "Tablet Lenovo"),
      ],
      elsewhere: ["Club de rol"],
    },
  ],
  unassigned: [dev("kitchen", "Tablet de la cocina")],
};
// `?withdrawn` sends Hugo away (D98): no access, no devices, still in the catalog.
if (params.has("withdrawn"))
  people.people = people.people.map((p) => (p.id === "hugo" ? { ...p, devices: [] } : p));
if (params.has("noroles")) {
  people.people = people.people.map((p) => ({ ...p, role: null, devices: [] }));
  people.unassigned = [
    dev("dev", "Portátil", true),
    dev("phone", "Móvil de Laura"),
    dev("tablet", "Tablet Lenovo"),
  ];
}

// `?stale` makes Hugo's request no longer applicable; `?elsewhere` has Laura
// has already accepted it when Ana tries to.
const minutesAgo = (m: number) => Date.now() - m * 60000;
const summary = (
  action: RequestSummary["action"],
  title: string,
  extra: Partial<RequestSummary> = {},
) => ({
  action,
  title,
  authors: [],
  year: null,
  publisher: null,
  format: null,
  location: null,
  ...extra,
});
const request = (
  id: string,
  person: string,
  deviceId: string,
  deviceName: string,
  at: number,
  s: RequestSummary,
  extra: Partial<ChangeRequest> = {},
): ChangeRequest => ({
  id,
  intent: "book.create",
  deviceId,
  deviceName,
  person,
  mine: params.has("reader") && person === "Hugo",
  at,
  note: null,
  status: "pending",
  resolvedBy: null,
  resolvedAt: null,
  summary: s,
  applies: true,
  editable: s.action === "add_book",
  ...extra,
});
const requests: ChangeRequest[] = [
  request(
    "r1",
    "Hugo",
    "hugo-phone",
    "Móvil de Hugo",
    minutesAgo(20),
    summary("add_book", "Tehanu", {
      authors: ["Ursula K. Le Guin"],
      year: 1990,
      publisher: "Minotauro",
      format: "paperback",
      location: "Cuarto de Hugo",
    }),
    {
      note: "Me lo regaló la abuela, ya lo tengo en la estantería.",
      applies: !params.has("stale"),
    },
  ),
  request(
    "r2",
    "Laura",
    "phone",
    "Móvil de Laura",
    minutesAgo(60 * 20),
    summary("delete_copy", "Dune", { format: "ebook" }),
  ),
  request(
    "r3",
    "Hugo",
    "hugo-phone",
    "Móvil de Hugo",
    minutesAgo(60 * 50),
    summary("add_book", "Cien años de soledad", {
      authors: ["Gabriel García Márquez"],
    }),
  ),
  request(
    "r4",
    "Hugo",
    "tablet",
    "Tablet Lenovo",
    minutesAgo(60 * 24 * 4),
    summary("delete_book", "Monopoly"),
    {
      status: "accepted",
      resolvedBy: "Ana",
      resolvedAt: minutesAgo(60 * 24 * 3),
    },
  ),
  request(
    "r5",
    "Laura",
    "phone",
    "Móvil de Laura",
    minutesAgo(60 * 24 * 16),
    summary("delete_copy", "El viaje de Chihiro"),
    {
      status: "rejected",
      resolvedBy: "Ana",
      resolvedAt: minutesAgo(60 * 24 * 15),
    },
  ),
];

// Marks (D68): `?who` asks «Who are you?» first; `?locked` is a device of nobody
// in a library with roles.
let claimed = !params.has("who") && !params.has("locked");
const myMarks = new Map<string, Mark>();
// One missing work already wished by "me", so the wish colour and «Lo desea…»
// wording both show without having to click anything first (D61).
myMarks.set(`work-${bookAt(13).editionId}`, {
  status: null,
  statusOn: null,
  rating: null,
  favorite: false,
  wished: true,
});
myMarks.set(`work-${bookAt(15).editionId}`, {
  status: null,
  statusOn: null,
  rating: null,
  favorite: false,
  wished: true,
});
const othersMarks: PersonMark[] = [
  {
    person: "Laura",
    status: "finished",
    statusOn: "2026-03-18",
    rating: 5,
    favorite: true,
    wished: false,
  },
  {
    person: "Hugo",
    status: "in_progress",
    statusOn: "2026-09-02",
    rating: null,
    favorite: false,
    wished: false,
  },
];

const matchesHolding = (holding: Holding | undefined, copyCount: number): boolean =>
  !holding || (holding === "owned" ? copyCount > 0 : copyCount === 0);

function mockCandidates(kind: Kind, query: string): Candidate[] {
  const candidate = (over: Partial<Candidate>): Candidate => ({
    kind,
    source: "open_library",
    externalId: null,
    title: query,
    subtitle: null,
    authors: [],
    contributors: [],
    publisher: null,
    publishedYear: null,
    language: null,
    coverUrl: null,
    isbn: null,
    barcode: null,
    number: null,
    playersMin: null,
    playersMax: null,
    durationMinutes: null,
    minAge: null,
    seasons: null,
    platform: null,
    format: null,
    existingEditionId: null,
    ...over,
  });

  if (kind === "book") {
    return [
      candidate({
        source: "open_library",
        externalId: "/works/OL893414W",
        title: `${query} (Open Library)`,
        authors: ["Frank Herbert"],
        publisher: "Gollancz",
        publishedYear: 1965,
        language: "en",
        isbn: "9780441013593",
        // No cover: a remote one here made the placeholder flaky in screenshots
        // (still loading, not yet erroring, when the shot was taken).
        coverUrl: null,
        // Same edition already in the catalog (bookAt(0), "Dune"), to show "Ya está".
        existingEditionId: bookAt(0).editionId,
      }),
      candidate({
        source: "inventaire",
        externalId: "wd:Q190192",
        title: `${query} (Inventaire)`,
        authors: ["Frank Herbert"],
        language: "es",
        coverUrl: "https://inventaire.io/img/entities/a936e0978d400fb13e0c0a2527d345a56b799e54",
      }),
      candidate({
        source: "wikidata",
        externalId: "wd:Q846578",
        title: `${query}: la casa Atreides`,
        authors: ["Brian Herbert", "Kevin J. Anderson"],
        publishedYear: 1999,
      }),
    ];
  }
  if (kind === "music") {
    return [
      candidate({
        source: "music_brainz",
        externalId: "mb-1",
        title: query,
        authors: ["Pink Floyd"],
        publisher: "Harvest",
        publishedYear: 1973,
        format: "vinyl",
      }),
    ];
  }
  // The other kinds only search Wikidata without the cloud or a key (D26): the
  // limited notice shows for them.
  return [
    candidate({
      source: "wikidata",
      externalId: "wd:Q1",
      title: query,
      authors: [kind === "film" || kind === "tv_series" ? "Denis Villeneuve" : "Studio"],
      publishedYear: 2021,
      durationMinutes: kind === "film" ? 155 : null,
    }),
  ];
}

mockIPC((cmd, args) => {
  switch (cmd) {
    case "list_books": {
      const filter = (args as { filter?: BookFilter }).filter ?? {};
      return books
        .map((b) => {
          const mark = myMarks.get(`work-${b.editionId}`);
          const wishedBy = wishedByFor(b.editionId);
          return {
            ...b,
            myStatus: mark?.status ?? null,
            myFavorite: mark?.favorite ?? false,
            myWished: mark?.wished ?? false,
            wishedBy,
            tags: tagNamesOf(b.editionId),
          };
        })
        .filter(
          (b) =>
            (!filter.kind || b.kind === filter.kind) &&
            matchesHolding(filter.holding, b.copyCount) &&
            (!filter.myStatuses?.length || filter.myStatuses.includes(b.myStatus ?? "none")) &&
            (!filter.myFavorites || b.myFavorite) &&
            (!filter.myWished || b.myWished) &&
            (!filter.othersWished || b.wishedBy.some((name) => name !== "me")) &&
            (!filter.genres?.length ||
              filter.genres.some((g) => (genresOfEdition[b.editionId] ?? []).includes(g))) &&
            (!filter.tags?.length ||
              filter.tags.some((id) => (workTagIds[b.editionId] ?? []).includes(id))),
        );
    }
    case "list_libraries":
      return libraries;
    // Backups (D86). The file pickers answer with fixed paths: a path containing
    // "encrypted" stands for a protected backup whose password is "mock".
    case "plugin:dialog|save":
      return "/mock/kosmotheka-backup";
    case "plugin:dialog|open":
      return "/mock/kosmotheka-encrypted.kosmotheka";
    case "export_backup":
    case "export_device_backup": {
      if ((args as { passphrase?: string }).passphrase === "") {
        throw { kind: "backup", reason: "passphrase_empty" };
      }
      return { libraries: libraries.length, books: 12, copies: 14, covers: 3 };
    }
    case "restore_backup": {
      const { path, passphrase } = args as { path: string; passphrase?: string };
      if (path.includes("encrypted")) {
        if (!passphrase) throw { kind: "backup", reason: "passphrase_needed" };
        if (passphrase !== "mock") throw { kind: "backup", reason: "wrong_passphrase" };
      }
      return { restored: [], skipped: [], covers: 0 };
    }
    case "duplicate_library": {
      const { id: fromId, name } = args as { id: string; name: string };
      const id = crypto.randomUUID();
      for (const library of libraries) library.active = false;
      const from = libraries.find((library) => library.id === fromId);
      libraries.push({
        id,
        name,
        works: from?.works ?? 0,
        active: true,
        created: { device: null, at: Date.now() },
        canShare: true,
      });
      return id;
    }
    case "library_devices":
      return lan.servers;
    case "sharing_overview":
      return lan.servers.map((device) => ({
        ...device,
        libraries: device.deviceId === "phone" ? [HOME, CLUB] : [HOME],
      }));
    case "open_pairing":
      return { code: "KOSMOTHEKA1:eyJkZXZpY2VJZCI6ImRldiJ9", expiresInSeconds: 582 };
    case "catalog_options":
      return {
        authors: [],
        publishers: [],
        owners: ["Tersoal", "Laura", "Hugo"],
        locations: ["Salón", "Estudio"],
      };
    case "lan_status":
      return lan;
    case "pair_with":
      return { deviceId: "pixel9", name: "Pixel 9" };
    case "pairing_owners":
      return {
        libraries: [HOME, CLUB],
        people: [
          {
            id: "ana",
            name: "Ana",
            me: true,
            roles: { [HOME]: "admin", [CLUB]: "admin" },
            devices: 2,
          },
          { id: "laura", name: "Laura", me: false, roles: { [HOME]: "editor" }, devices: 1 },
          {
            id: "hugo",
            name: "Hugo",
            me: false,
            roles: { [HOME]: "reader", [CLUB]: "reader" },
            devices: 2,
          },
        ],
      };
    case "give_access":
      return null;
    case "sync_with":
      if ((args as { deviceId: string }).deviceId === "tablet") {
        throw { kind: "sync", reason: "other_version" };
      }
      return {
        received: { received: 0, known: 0, rejected: 0, skippedChanges: 0 },
        sent: 0,
        takenUp: false,
        removed: [],
      };
    case "is_mobile":
      return new URLSearchParams(window.location.search).has("mobile");
    case "welcome_seen":
      return mockWelcomeSeen;
    case "set_welcome_seen":
      mockWelcomeSeen = (args as { version: number }).version;
      return null;
    case "app_lock_enabled":
      return mockAppLock;
    case "set_app_lock":
      mockAppLock = (args as { enabled: boolean }).enabled;
      return null;
    case "app_lock_after":
      return mockAppLockAfter;
    case "set_app_lock_after":
      mockAppLockAfter = (args as { minutes: number }).minutes;
      return null;
    // Any ISBN starting with 97884 is "not owned"; lookups always fail, as if offline.
    case "check_isbn": {
      const isbn = (args as { isbn: string }).isbn.replace(/\D/g, "");
      return isbn.startsWith("97884")
        ? { isbn13: isbn, books: [], missing: [] }
        : { isbn13: isbn, books: [bookAt(0)], missing: [] };
    }
    case "lookup_isbn":
      throw new Error("offline");
    case "search_works": {
      const { kind, query } = args as { kind: Kind; query: string };
      if (query.trim().length < 2) return { candidates: [], warnings: [] };
      return { candidates: mockCandidates(kind, query), warnings: [] };
    }
    case "lookup_code": {
      const { code } = args as { code: string };
      const digits = code.replace(/\D/g, "");
      // A 978/979 EAN-13 is an ISBN: always a book. One sample EAN is a known
      // album; any other code has no clear kind, showing the "¿Qué es?" step.
      if (digits.length === 13 && /^97[89]/.test(digits)) {
        // A code lookup only returns sources for that one edition (unlike a
        // title search, which can turn up other related works).
        const candidates = mockCandidates("book", "Dune").slice(0, 2);
        return {
          kind: "book",
          candidates,
          warnings: [],
          existingEditionId: candidates[0]?.existingEditionId ?? null,
        };
      }
      if (digits === "5099902988371") {
        return {
          kind: "music",
          candidates: mockCandidates("music", "The Dark Side of the Moon"),
          warnings: [],
          existingEditionId: null,
        };
      }
      return { kind: null, candidates: [], warnings: [], existingEditionId: null };
    }
    case "work_marks": {
      const { workId } = args as { workId: string };
      return {
        mine: myMarks.get(workId) ?? null,
        others: workId.endsWith("1") ? othersMarks : othersMarks.slice(0, 1),
        canMark: claimed,
        mayClaim: !claimed && !params.has("locked"),
      };
    }
    case "set_mark": {
      const { workId, mark } = args as { workId: string; mark: Mark };
      myMarks.set(workId, mark);
      return null;
    }
    case "claim_candidates":
      return [
        { id: "laura", name: "Laura", copies: 12 },
        { id: "hugo", name: "Hugo", copies: 5 },
      ];
    case "claim_person":
      claimed = true;
      return null;
    case "get_book": {
      const id = (args as { editionId: string }).editionId;
      const found = books.find((b) => b.editionId === id) ?? bookAt(0);
      // Plausible per-kind extras (D68-ish), so every kind's edit and copy forms have
      // something to show; kinds without a field keep it null, as the engine would.
      const KIND_EXTRAS: Partial<
        Record<
          Kind,
          {
            contributors?: { name: string; role: string }[];
            number?: string;
            playersMin?: number;
            playersMax?: number;
            durationMinutes?: number;
            minAge?: number;
            seasons?: number;
            barcode?: string;
          }
        >
      > = {
        comic: {
          contributors: [
            { name: "Alan Moore", role: "writer" },
            { name: "Dave Gibbons", role: "illustrator" },
          ],
          number: "1",
          barcode: "9788467905236",
        },
        boardgame: {
          contributors: [{ name: "Klaus Teuber", role: "designer" }],
          playersMin: 3,
          playersMax: 4,
          durationMinutes: 75,
          minAge: 10,
          barcode: "4001504036136",
        },
        film: {
          contributors: [{ name: "Denis Villeneuve", role: "director" }],
          durationMinutes: 163,
          barcode: "8420266008358",
        },
        tv_series: { seasons: 3, barcode: "7321900000000" },
        videogame: {
          contributors: [{ name: "Nintendo EPD", role: "studio" }],
          barcode: "0045496596439",
        },
        music: {
          contributors: [{ name: "The Beatles", role: "artist" }],
          barcode: "5099969944924",
        },
      };
      const extras = KIND_EXTRAS[found.kind] ?? {};
      return {
        ...found,
        workId: `work-${found.editionId}`,
        language: found.kind === "book" ? "es" : null,
        googleBooksId: found.kind === "book" ? "abc" : null,
        openLibraryId: null,
        barcode: extras.barcode ?? null,
        notes:
          found.editionId === bookAt(0).editionId
            ? "Completo, con la caja original. Falta una vela."
            : null,
        contributors: extras.contributors ?? [],
        number: extras.number ?? null,
        playersMin: extras.playersMin ?? null,
        playersMax: extras.playersMax ?? null,
        durationMinutes: extras.durationMinutes ?? null,
        minAge: extras.minAge ?? null,
        seasons: extras.seasons ?? null,
        copies: found.formats.map((format, i) => {
          const digital = ["ebook", "pdf", "audiobook", "digital"].includes(format);
          return {
            id: `copy-${i}`,
            owner: found.owners[i] ?? found.owners[0] ?? null,
            format,
            quantity: 1,
            location: !digital
              ? "Salón, balda 3"
              : found.kind === "videogame"
                ? "Nintendo eShop"
                : found.kind === "tv_series"
                  ? "Filmin"
                  : null,
            acquiredOn: null,
            platform: found.kind === "videogame" ? (i === 0 ? "Switch" : "PS5") : null,
            seasons: found.kind === "tv_series" ? (i === 0 ? "1-2" : "3") : null,
            canDelete: !params.has("reader"),
          };
        }),
        canDelete: !params.has("reader"),
        genres: genresOfEdition[found.editionId] ?? [],
        tags: tagNamesOf(found.editionId),
        wishedBy: wishedByFor(found.editionId),
        series: seriesOfEdition[found.editionId] ?? [],
        requires: requiresOfEdition[found.editionId] ?? [],
        requiredBy: requiredByOfEdition[found.editionId] ?? [],
      };
    }
    case "create_book":
      return { editionId: "new-edition-id", existingEdition: false, coverFailed: false };
    case "update_edition": {
      const { editionId, edition } = args as {
        editionId: string;
        edition: { genres?: string[]; tags?: string[] };
      };
      genresOfEdition[editionId] = edition.genres ?? [];
      workTagIds[editionId] = (edition.tags ?? []).map((name) => findOrCreateTag(name).id);
      return { coverFailed: false };
    }
    case "add_copy":
      return "new-copy-id";
    case "update_copy":
    case "delete_copy":
    case "delete_book":
      return null;
    case "list_series":
      return Object.values(seriesById).map((s) => ({
        id: s.id,
        name: s.name,
        ordered: s.ordered,
        works: s.entries.length,
        missing: s.entries.filter((e) => e.copies === 0).length,
      }));
    case "get_series": {
      const id = (args as { id: string }).id;
      return seriesById[id] ?? seriesById[DUNE_SERIES];
    }
    case "create_series":
      return "series-new";
    case "update_series":
    case "delete_series":
    case "remove_from_series":
    case "reorder_series":
    case "add_requirement":
    case "remove_requirement":
      return null;
    case "add_to_series":
      return "entry-new";
    case "list_tags":
      return mockTags
        .map((t) => ({ id: t.id, name: t.name, works: tagWorksCount(t.id) }))
        .sort((a, b) => a.name.localeCompare(b.name));
    case "rename_tag": {
      const { id, name } = args as { id: string; name: string };
      const trimmed = name.trim();
      const target = mockTags.find(
        (t) => t.id !== id && t.name.toLowerCase() === trimmed.toLowerCase(),
      );
      if (target) {
        // Merge: move this tag's work links onto the existing one, then drop it.
        for (const [editionId, ids] of Object.entries(workTagIds)) {
          if (!ids.includes(id)) continue;
          workTagIds[editionId] = Array.from(new Set([...ids.filter((i) => i !== id), target.id]));
        }
        mockTags = mockTags.filter((t) => t.id !== id);
      } else {
        const tag = mockTags.find((t) => t.id === id);
        if (tag) tag.name = trimmed;
      }
      return null;
    }
    case "delete_tag": {
      const { id } = args as { id: string };
      mockTags = mockTags.filter((t) => t.id !== id);
      for (const editionId of Object.keys(workTagIds)) {
        workTagIds[editionId] = (workTagIds[editionId] ?? []).filter((i) => i !== id);
      }
      return null;
    }
    case "genres_of_kind":
      return GENRES_BY_KIND[(args as { kind: Kind }).kind];
    case "add_missing":
      return { editionId: "new-missing-edition-id", existingEdition: false, coverFailed: false };
    case "request_book":
    case "request_missing":
    case "request_add_copy":
    case "request_delete_copy":
    case "request_delete_book":
      return { requested: true };
    // `?notice` shows a change of this device that became a request on syncing.
    case "request_notices":
      // `?notice` (no value, or any value besides an account notice's) is this
      // page's own flag; `?notice=device_removed`/`signed_out_password_changed`
      // is the account notice instead (D77), set on `mockAccount` above.
      return params.has("notice") && !mockAccount.notice
        ? [{ ...requests[1], mine: true, summary: summary("edit", "Dune") }]
        : [];
    case "dismiss_request_notices":
      return null;
    case "book_history":
      return [];
    case "list_people":
      return people;
    case "list_requests":
      return requests.map((r) => ({ ...r, summary: { ...r.summary } }));
    case "accept_request":
    case "reject_request":
    case "withdraw_request": {
      const found = requests.find((r) => r.id === (args as { id: string }).id);
      if (found?.id === "r1" && params.has("elsewhere")) {
        Object.assign(found, { status: "accepted", resolvedBy: "Laura", resolvedAt: Date.now() });
        throw { kind: "permission", reason: "request_resolved" };
      }
      if (found)
        Object.assign(found, {
          status:
            cmd === "accept_request"
              ? "accepted"
              : cmd === "reject_request"
                ? "rejected"
                : "withdrawn",
          resolvedBy: "Ana",
          resolvedAt: Date.now(),
        });
      return null;
    }
    case "requested_book":
      return {
        title: "Tehanu",
        subtitle: null,
        authors: ["Ursula K. Le Guin"],
        publisher: "Minotauro",
        publishedYear: 1990,
        language: "es",
        isbn: null,
        coverUrl: null,
        googleBooksId: null,
        openLibraryId: null,
        copy: {
          owner: null,
          format: "paperback",
          quantity: 1,
          location: "Cuarto de Hugo",
          acquiredOn: null,
        },
      };
    case "known_people":
      return [{ id: "marcos", name: "Marcos", libraries: ["Club de rol"] }];
    case "get_settings":
      return mockSettings;
    case "save_settings":
      mockSettings = (args as { settings: typeof mockSettings }).settings;
      return null;
    case "account_status":
      return mockAccount;
    case "dismiss_account_notice":
      mockAccount = { ...mockAccount, notice: null };
      return null;
    case "set_account_server": {
      if (offline()) throw { kind: "account", reason: "server_unavailable" };
      const { url } = args as { url: string | null };
      if (url === null) {
        mockAccount = { ...mockAccount, server: "https://api.kosmotheka.app", official: true };
        return null;
      }
      try {
        new URL(url);
      } catch {
        throw { kind: "account", reason: "invalid_server" };
      }
      mockAccount = { ...mockAccount, server: url, official: false };
      return null;
    }
    case "account_server_info": {
      if (offline()) throw { kind: "account", reason: "server_unavailable" };
      const info: ServerInfo = {
        name: "Kosmotheka",
        apiVersion: 1,
        registration: true,
        signInWithGoogle: false,
        twoFactor: true,
      };
      return info;
    }
    case "register_account":
      if (offline()) throw { kind: "account", reason: "server_unavailable" };
      return null;
    case "confirm_account_email": {
      if (offline()) throw { kind: "account", reason: "server_unavailable" };
      const { code } = args as { code: string };
      if (code === "000000") throw { kind: "account", reason: "invalid_code" };
      return null;
    }
    case "resend_account_code":
      if (offline()) throw { kind: "account", reason: "server_unavailable" };
      return null;
    case "sign_in": {
      if (offline()) throw { kind: "account", reason: "server_unavailable" };
      const { email, password, totpCode } = args as {
        email: string;
        password: string;
        totpCode: string | null;
      };
      if (password === "wrong") throw { kind: "account", reason: "invalid_credentials" };
      if (email.startsWith("2fa@")) {
        if (!totpCode) throw { kind: "account", reason: "two_factor_required" };
        if (totpCode === "000000") throw { kind: "account", reason: "invalid_two_factor_code" };
      }
      const user: AccountUser = { id: "ana", email, name: "Ana Ruiz", locale: "es" };
      mockAccount = { ...mockAccount, signedIn: true, user, emailHint: email, notice: null };
      return user;
    }
    case "sign_out":
      mockAccount = { ...mockAccount, signedIn: false, user: null };
      return null;
    case "forgot_password":
      if (offline()) throw { kind: "account", reason: "server_unavailable" };
      return null;
    case "reset_password": {
      if (offline()) throw { kind: "account", reason: "server_unavailable" };
      const { code } = args as { code: string };
      if (code === "000000") throw { kind: "account", reason: "invalid_code" };
      mockAccount = { ...mockAccount, signedIn: false, user: null };
      return null;
    }

    // Account screen, second factor, devices and handing the device away (hito 7.1) ---

    case "get_account": {
      if (!mockAccount.signedIn) throw { kind: "account", reason: "not_signed_in" };
      if (offline()) throw { kind: "account", reason: "server_unavailable" };
      return mockAccountDetails;
    }
    case "update_account": {
      if (!mockAccount.signedIn) throw { kind: "account", reason: "not_signed_in" };
      if (offline()) throw { kind: "account", reason: "server_unavailable" };
      const { name, locale } = args as { name: string | null; locale: string | null };
      if (name) mockAccountDetails = { ...mockAccountDetails, name };
      if (locale) mockAccountDetails = { ...mockAccountDetails, locale };
      if (mockAccount.user && (name || locale)) {
        mockAccount = {
          ...mockAccount,
          user: {
            ...mockAccount.user,
            name: name ?? mockAccount.user.name,
            locale: locale ?? mockAccount.user.locale,
          },
        };
      }
      return mockAccountDetails;
    }
    case "change_password": {
      if (offline()) throw { kind: "account", reason: "server_unavailable" };
      const { currentPassword } = args as { currentPassword: string };
      if (currentPassword === "wrong") throw { kind: "account", reason: "invalid_credentials" };
      return null;
    }
    case "delete_account": {
      if (offline()) throw { kind: "account", reason: "server_unavailable" };
      const { password } = args as { password: string };
      if (password === "wrong") throw { kind: "account", reason: "invalid_credentials" };
      mockAccount = { ...mockAccount, signedIn: false, user: null };
      return null;
    }
    case "account_devices": {
      if (!mockAccount.signedIn) throw { kind: "account", reason: "not_signed_in" };
      if (offline()) throw { kind: "account", reason: "server_unavailable" };
      return mockDevices;
    }
    case "remove_account_device": {
      const { id } = args as { id: string };
      mockDevices = mockDevices.map((d) =>
        d.id === id ? { ...d, removed: true, canRestore: true } : d,
      );
      return null;
    }
    case "restore_account_device": {
      const { id } = args as { id: string };
      const device = mockDevices.find((d) => d.id === id);
      if (device && !device.canRestore) throw { kind: "account", reason: "device_already_reset" };
      mockDevices = mockDevices.map((d) =>
        d.id === id ? { ...d, removed: false, canRestore: false } : d,
      );
      return null;
    }
    case "two_factor_setup": {
      if (offline()) throw { kind: "account", reason: "server_unavailable" };
      const setup: TwoFactorSetup = {
        secret: "JBSWY3DPEHPK3PXP",
        otpauthUri:
          "otpauth://totp/Kosmotheka:ana.ruiz%40correo.es?secret=JBSWY3DPEHPK3PXP&issuer=Kosmotheka",
      };
      return setup;
    }
    case "two_factor_enable": {
      const { code } = args as { code: string };
      if (code === "000000") throw { kind: "account", reason: "invalid_two_factor_code" };
      mockAccountDetails = { ...mockAccountDetails, twoFactorEnabled: true, backupCodesLeft: 10 };
      return ACCOUNT_BACKUP_CODES;
    }
    case "two_factor_disable": {
      const { password, code } = args as { password: string; code: string };
      if (password === "wrong") throw { kind: "account", reason: "invalid_credentials" };
      if (code === "000000") throw { kind: "account", reason: "invalid_two_factor_code" };
      mockAccountDetails = { ...mockAccountDetails, twoFactorEnabled: false, backupCodesLeft: 0 };
      return null;
    }
    case "regenerate_backup_codes": {
      const { password, code } = args as { password: string; code: string };
      if (password === "wrong") throw { kind: "account", reason: "invalid_credentials" };
      if (code === "000000") throw { kind: "account", reason: "invalid_two_factor_code" };
      mockAccountDetails = { ...mockAccountDetails, backupCodesLeft: 10 };
      return ACCOUNT_BACKUP_CODES;
    }
    case "cloud_connection":
      return { live: mockAccount.signedIn, available: mockAccount.signedIn };
    case "cloud_events_wake":
      return null;
    case "cloud_status":
      return libraries.map((library) => {
        const linked = cloudLinked.has(library.id);
        const unsigned = cloudMode !== null && library.id === RECIPES;
        return {
          library: library.id,
          cloudLibrary: linked ? `cloud-${library.id}` : null,
          cloudServer: linked ? "https://api.kosmotheka.app" : null,
          linkedHere: linked,
          signedIn: mockAccount.signedIn,
          mayShare: library.canShare,
          unsignedHistory: unsigned,
          // Only whoever administers it uploads (D76); `?reader` is Hugo, who does not.
          canUpload: mockAccount.signedIn && !params.has("reader") && !unsigned && !linked,
        };
      });
    case "upload_to_cloud": {
      if (!mockAccount.signedIn) throw { kind: "cloud", reason: "not_signed_in" };
      const { libraryId } = args as { libraryId: string };
      if (cloudMode !== null && libraryId === RECIPES) {
        throw { kind: "cloud", reason: "unsigned_history" };
      }
      cloudLinked.add(libraryId);
      return {
        library: libraryId,
        cloudLibrary: `cloud-${libraryId}`,
        outcome: "synced",
        received: 0,
        sent: 12,
        rejected: 0,
      };
    }
    case "sync_cloud": {
      if (!mockAccount.signedIn) throw { kind: "cloud", reason: "not_signed_in" };
      if (cloudMode === "offline") throw { kind: "account", reason: "server_unavailable" };
      const report = (library: string, outcome: string) => ({
        library,
        cloudLibrary: outcome === "synced" ? `cloud-${library}` : null,
        outcome,
        received: 0,
        sent: 0,
        rejected: 0,
      });
      const reports = [...cloudLinked].map((id) =>
        report(id, cloudMode === "missing" && id === HOME ? "missing_in_cloud" : "synced"),
      );
      if (cloudMode === "conflict") reports.push(report(CLUB, "library_conflict"));
      if (cloudMode === "deleted") reports.push(report("deleted-library", "deleted_here"));
      return reports;
    }
    case "invite_status": {
      // `?blocker=not_in_cloud` (or no_roles, no_cloud_power, server_without_invitations, not_admin,
      // not_signed_in) shows each reason a library cannot invite.
      const asked = params.get("blocker");
      const blocker =
        asked ??
        (!mockAccount.signedIn ? "not_signed_in" : params.has("reader") ? "not_admin" : null);
      return { canInvite: blocker === null, blocker };
    }
    case "invite_to_library": {
      const { request } = args as {
        request: { person: { type: string; member?: string }; kind: string };
      };
      const isNew = request.person.type === "new";
      const email = (request as { email?: string | null }).email;
      if (request.kind === "email" && !email?.includes("@"))
        throw { kind: "account", reason: "invalid_email" };
      const member = isNew ? `new-${mockInvitations.length + 1}` : (request.person.member ?? "");
      const id = `invitation-${mockInvitations.length + 1}`;
      mockInvitations.push({
        id,
        kind: request.kind,
        memberId: member,
        state: "pending",
        createdAt: new Date().toISOString(),
        expiresAt: new Date(Date.now() + 24 * 3600 * 1000).toISOString(),
        acceptedAt: null,
      });
      // `?invitefails`: the person is added but the invitation is not made (the server refused).
      if (params.has("invitefails")) {
        mockInvitations.pop();
        return { member, personCreated: isNew, failed: "too_many_requests", invitation: null };
      }
      return {
        member,
        personCreated: isNew,
        failed: null,
        invitation: {
          id,
          kind: request.kind,
          memberId: member,
          expiresAt: new Date(Date.now() + 24 * 3600 * 1000).toISOString(),
          link: request.kind === "link" ? `https://kosmotheka.app/i/${MOCK_TOKEN}` : null,
        },
      };
    }
    case "list_invitations":
      return mockInvitations;
    case "revoke_invitation": {
      const { invitationId } = args as { invitationId: string };
      for (const item of mockInvitations) if (item.id === invitationId) item.state = "revoked";
      return null;
    }
    case "open_invitation": {
      const { link } = args as { link: string };
      // The text of the link picks the answer, to see every error: «used», «expired», «revoked»,
      // «otheraccount», «otherserver», «notfound»; anything without `/i/` is not a link.
      if (!link.includes("/i/")) throw { kind: "cloud", reason: "invitation_bad_link" };
      const failures: Record<string, string> = {
        used: "invitation_used",
        expired: "invitation_expired",
        revoked: "invitation_revoked",
        otheraccount: "invitation_not_for_this_account",
        otherserver: "invitation_other_server",
        notfound: "invitation_not_found",
      };
      const failure = Object.keys(failures).find((word) => link.includes(word));
      if (failure) {
        const reason = failures[failure];
        throw { kind: reason === "invitation_other_server" ? "cloud" : "account", reason };
      }
      mockPendingInvitation = {
        libraryName: "Casa",
        kind: "link",
        expiresAt: new Date(Date.now() + 24 * 3600 * 1000).toISOString(),
        signedIn: mockAccount.signedIn,
      };
      return mockPendingInvitation;
    }
    case "pending_invitation":
      return mockPendingInvitation;
    case "discard_pending_invitation":
      mockPendingInvitation = null;
      return null;
    case "accept_invitation": {
      if (!mockPendingInvitation) throw { kind: "cloud", reason: "invitation_no_pending" };
      if (!mockAccount.signedIn) throw { kind: "cloud", reason: "not_signed_in" };
      mockPendingInvitation = null;
      // `?joinlater`: used, but the library has not come down yet.
      const later = params.has("joinlater");
      return {
        cloudLibrary: "cloud-casa",
        library: later ? null : HOME,
        libraryName: "Casa",
        reports: [],
        syncError: later ? "server_unavailable" : null,
      };
    }
    case "give_device_away":
      // The real app reloads from the `device-reset` event Rust emits; the mock
      // has no Rust side to emit it, so it reloads the page itself instead.
      setTimeout(() => window.location.reload(), 50);
      return null;

    default:
      return null;
  }
});
