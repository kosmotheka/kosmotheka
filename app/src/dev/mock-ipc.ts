// Preview mode for design work: open the Vite dev server with `?mock` in a
// normal browser and the Rust commands answer with sample data. Only loaded
// in development (see main.tsx); never part of a build.
import { mockIPC } from "@tauri-apps/api/mocks";

import type {
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
  RequestSummary,
  Wish,
} from "@/lib/api";

const HOME = "00000000-0000-7000-8000-000000000001";
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
];
// Fundación keeps a pocket copy and an ISBN, for the change-of-kind screenshots
// (book -> comic): the ISBN becomes a barcode and "Bolsillo" needs a new format.
books[2] = { ...books[2], isbn13: "9788497596824" };
// Dune shows a filled ISBN row on its own work page.
books[0] = { ...books[0], isbn13: "9788497596848" };

const wishes: Wish[] = [
  {
    id: "w1",
    title: "Hijos de Dune",
    author: "Frank Herbert",
    isbn13: null,
    note: null,
    coverUrl: null,
    createdAt: "2026-09-20T10:00:00Z",
  },
  {
    id: "w2",
    title: "Wingspan",
    author: null,
    isbn13: null,
    note: "La pide Laura",
    coverUrl: null,
    createdAt: "2026-09-21T10:00:00Z",
  },
];

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
const dev = (id: string, name: string, thisDevice = false) => ({ id, name, thisDevice });
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
      devices: [dev("dev", "Portátil", !params.has("reader")), dev("pixel", "Pixel 8")],
      elsewhere: ["Club de rol"],
    },
    {
      id: "laura",
      name: "Laura",
      role: "editor",
      canShare: true,
      devices: [dev("phone", "Móvil de Laura")],
      elsewhere: [],
    },
    {
      id: "hugo",
      name: "Hugo",
      role: "reader",
      canShare: false,
      devices: [
        dev("hugo-phone", "Móvil de Hugo", params.has("reader")),
        dev("tablet", "Tablet Lenovo"),
      ],
      elsewhere: ["Club de rol"],
    },
  ],
  unassigned: [dev("kitchen", "Tablet de la cocina")],
};
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
        // Same edition already in the catalog (books[0], "Dune"), to show "Ya está".
        existingEditionId: books[0].editionId,
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
          return {
            ...b,
            myStatus: mark?.status ?? null,
            myFavorite: mark?.favorite ?? false,
            myWished: mark?.wished ?? false,
          };
        })
        .filter(
          (b) =>
            (!filter.kind || b.kind === filter.kind) &&
            matchesHolding(filter.holding, b.copyCount) &&
            (!filter.myStatuses?.length || filter.myStatuses.includes(b.myStatus ?? "none")) &&
            (!filter.myFavorites || b.myFavorite) &&
            (!filter.myWished || b.myWished),
        );
    }
    case "list_libraries":
      return libraries;
    case "library_devices":
      return lan.servers;
    case "sharing_overview":
      return lan.servers.map((device) => ({
        ...device,
        libraries: device.deviceId === "phone" ? [HOME, CLUB] : [HOME],
      }));
    case "open_pairing":
      return { code: "KOSMOTHEKA1:eyJkZXZpY2VJZCI6ImRldiJ9", expiresInSeconds: 582 };
    case "list_wishes":
      return wishes;
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
    // Any ISBN starting with 97884 is "not owned"; lookups always fail, as if offline.
    case "check_isbn": {
      const isbn = String((args as { isbn: string }).isbn).replace(/\D/g, "");
      return isbn.startsWith("97884")
        ? { isbn13: isbn, books: [], wishes: [] }
        : { isbn13: isbn, books: [books[0]], wishes: [] };
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
          existingEditionId: candidates[0].existingEditionId,
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
      const found = books.find((b) => b.editionId === id) ?? books[0];
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
          found.editionId === books[0].editionId
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
        genres: found.kind === "book" ? ["fantasy", "science_fiction"] : [],
        tags: ["Favorite"],
        series:
          found.kind === "book"
            ? [{ seriesId: "series-1", name: "Dune", ordered: true, position: 1 }]
            : [],
        requires: [],
        requiredBy: [],
      };
    }
    case "create_book":
      return { editionId: "new-edition-id", existingEdition: false, coverFailed: false };
    case "update_edition":
      return { coverFailed: false };
    case "add_copy":
      return "new-copy-id";
    case "update_copy":
    case "delete_copy":
    case "delete_book":
      return null;
    case "list_series":
      return [{ id: "series-1", name: "Dune", ordered: true, works: 2, missing: 1 }];
    case "get_series":
      return {
        id: "series-1",
        name: "Dune",
        ordered: true,
        entries: [
          {
            entryId: "entry-1",
            workId: `work-${books[0].editionId}`,
            editionId: books[0].editionId,
            title: books[0].title,
            kind: books[0].kind,
            cover: books[0].cover,
            position: 1,
            copies: books[0].copyCount,
          },
          {
            entryId: "entry-2",
            workId: `work-${books[1].editionId}`,
            editionId: books[1].editionId,
            title: books[1].title,
            kind: books[1].kind,
            cover: books[1].cover,
            position: 2,
            copies: 0,
          },
        ],
      };
    case "create_series":
      return "series-new";
    case "update_series":
    case "delete_series":
    case "set_series_entry_position":
    case "remove_from_series":
    case "reorder_series":
    case "add_requirement":
    case "remove_requirement":
      return null;
    case "add_to_series":
      return "entry-new";
    case "list_tags":
      return [
        { id: "tag-1", name: "Favorite", works: 3 },
        { id: "tag-2", name: "Gift", works: 1 },
      ];
    case "rename_tag":
    case "delete_tag":
      return null;
    case "genres_of_kind":
      return ["fantasy", "science_fiction", "adventure"];
    case "add_missing":
      return null;
    case "request_book":
    case "request_missing":
    case "request_delete_copy":
    case "request_delete_book":
      return { requested: true };
    // `?notice` shows a change of this device that became a request on syncing.
    case "request_notices":
      return params.has("notice")
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
      return { googleBooksApiKey: null };
    default:
      return null;
  }
});
