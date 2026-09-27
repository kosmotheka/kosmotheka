// Preview mode for design work: open the Vite dev server with `?mock` in a
// normal browser and the Rust commands answer with sample data. Only loaded
// in development (see main.tsx); never part of a build.
import { mockIPC } from "@tauri-apps/api/mocks";

import type {
  BookSummary,
  ChangeRequest,
  LanStatus,
  LibrarySummary,
  People,
  RequestSummary,
  Wish,
} from "@/lib/api";

const HOME = "00000000-0000-7000-8000-000000000001";
const CLUB = "01997c7e-0000-7000-8000-00000000c1b0";

const libraries: LibrarySummary[] = [
  { id: HOME, name: "Casa", works: 7, active: true, created: null },
  {
    id: CLUB,
    name: "Club de rol",
    works: 3,
    active: false,
    created: { device: "Móvil de Laura", at: Date.parse("2026-09-25T18:00:00Z") },
  },
];

const book = (
  n: number,
  title: string,
  authors: string[],
  year: number,
  formats: BookSummary["formats"],
  owners: string[],
): BookSummary => ({
  libraryId: HOME,
  editionId: `01997c7e-0000-7000-8000-${String(n).padStart(12, "0")}`,
  title,
  subtitle: null,
  authors,
  publisher: "Debolsillo",
  publishedYear: year,
  isbn13: null,
  cover: null,
  copyCount: formats.length,
  formats,
  owners,
});

const books: BookSummary[] = [
  book(1, "Dune", ["Frank Herbert"], 1965, ["paperback", "ebook"], ["Tersoal", "Laura"]),
  book(2, "El mesías de Dune", ["Frank Herbert"], 1969, ["paperback"], ["Tersoal"]),
  book(3, "Fundación", ["Isaac Asimov"], 1951, ["pocket"], ["Tersoal"]),
  book(4, "La mano izquierda de la oscuridad", ["Ursula K. Le Guin"], 1969, ["ebook"], ["Laura"]),
  book(5, "El nombre del viento", ["Patrick Rothfuss"], 2007, ["hardcover"], ["Tersoal"]),
  book(6, "Crónicas de la Dragonlance", ["Margaret Weis", "Tracy Hickman"], 1987, ["paperback"], ["Hugo"]),
  book(7, "Vampiro: la mascarada", ["Mark Rein-Hagen"], 1991, ["hardcover", "other"], ["Tersoal"]),
];

const wishes: Wish[] = [
  { id: "w1", title: "Hijos de Dune", author: "Frank Herbert", isbn13: null, note: null, coverUrl: null, createdAt: "2026-09-20T10:00:00Z" },
  { id: "w2", title: "Wingspan", author: null, isbn13: null, note: "La pide Laura", coverUrl: null, createdAt: "2026-09-21T10:00:00Z" },
];

const lan: LanStatus = {
  deviceId: "dev",
  deviceName: "Portátil",
  servingPort: null,
  addresses: [],
  pairingOpen: false,
  servers: [
    { deviceId: "phone", name: "Móvil de Laura", pairedAt: "2026-09-24T10:00:00Z", lastSyncAt: new Date(Date.now() - 2 * 60000).toISOString() },
    { deviceId: "tablet", name: "Tablet Lenovo", pairedAt: "2026-09-24T10:00:00Z", lastSyncAt: null },
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
    { id: "ana", name: "Ana", role: "admin", canShare: true, devices: [dev("dev", "Portátil", !params.has("reader")), dev("pixel", "Pixel 8")], elsewhere: ["Club de rol"] },
    { id: "laura", name: "Laura", role: "editor", canShare: true, devices: [dev("phone", "Móvil de Laura")], elsewhere: [] },
    { id: "hugo", name: "Hugo", role: "reader", canShare: false, devices: [dev("hugo-phone", "Móvil de Hugo", params.has("reader")), dev("tablet", "Tablet Lenovo")], elsewhere: ["Club de rol"] },
  ],
  unassigned: [dev("kitchen", "Tablet de la cocina")],
};
if (params.has("noroles")) {
  people.people = people.people.map((p) => ({ ...p, role: null, devices: [] }));
  people.unassigned = [dev("dev", "Portátil", true), dev("phone", "Móvil de Laura"), dev("tablet", "Tablet Lenovo")];
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

mockIPC((cmd, args) => {
  switch (cmd) {
    case "list_books":
      return books;
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
      return { authors: [], publishers: [], owners: ["Tersoal", "Laura", "Hugo"], locations: ["Salón", "Estudio"] };
    case "lan_status":
      return lan;
    case "pair_with":
      return { deviceId: "pixel9", name: "Pixel 9" };
    case "pairing_owners":
      return {
        libraries: [HOME, CLUB],
        people: [
          { id: "ana", name: "Ana", me: true, roles: { [HOME]: "admin", [CLUB]: "admin" }, devices: 2 },
          { id: "laura", name: "Laura", me: false, roles: { [HOME]: "editor" }, devices: 1 },
          { id: "hugo", name: "Hugo", me: false, roles: { [HOME]: "reader", [CLUB]: "reader" }, devices: 2 },
        ],
      };
    case "give_access":
      return null;
    case "sync_with":
      if ((args as { deviceId: string }).deviceId === "tablet") {
        throw { kind: "sync", reason: "other_version" };
      }
      return { received: { received: 0, known: 0, rejected: 0, skippedChanges: 0 }, sent: 0, takenUp: false, removed: [] };
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
    case "get_book": {
      const id = (args as { editionId: string }).editionId;
      const found = books.find((b) => b.editionId === id) ?? books[0];
      return {
        ...found,
        workId: "work",
        language: "es",
        googleBooksId: "abc",
        openLibraryId: null,
        copies: found.formats.map((format, i) => ({
          id: `copy-${i}`,
          owner: found.owners[i] ?? found.owners[0] ?? null,
          format,
          quantity: 1,
          location: format === "ebook" ? null : "Salón, balda 3",
          acquiredOn: null,
          canDelete: !params.has("reader"),
        })),
        canDelete: !params.has("reader"),
      };
    }
    case "request_book":
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
