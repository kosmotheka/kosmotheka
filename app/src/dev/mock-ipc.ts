// Preview mode for design work: open the Vite dev server with `?mock` in a
// normal browser and the Rust commands answer with sample data. Only loaded
// in development (see main.tsx); never part of a build.
import { mockIPC } from "@tauri-apps/api/mocks";

import type { BookSummary, LanStatus, LibrarySummary, Wish } from "@/lib/api";

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
        })),
      };
    }
    case "book_history":
      return [];
    case "get_settings":
      return { googleBooksApiKey: null };
    default:
      return null;
  }
});
