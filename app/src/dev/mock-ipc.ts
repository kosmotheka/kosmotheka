// Preview mode for design work: open the Vite dev server with `?mock` in a
// normal browser and the Rust commands answer with sample data. Only loaded
// in development (see main.tsx); never part of a build.
import { mockIPC } from "@tauri-apps/api/mocks";

import type { BookSummary, LanStatus, Wish } from "@/lib/api";

const book = (
  n: number,
  title: string,
  authors: string[],
  year: number,
  formats: BookSummary["formats"],
  owners: string[],
): BookSummary => ({
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
  servers: [{ deviceId: "phone", name: "Móvil de Laura", pairedAt: "2026-09-24T10:00:00Z", lastSyncAt: new Date(Date.now() - 2 * 60000).toISOString() }],
  clients: [],
};

mockIPC((cmd, args) => {
  switch (cmd) {
    case "list_books":
      return books;
    case "list_wishes":
      return wishes;
    case "catalog_options":
      return { authors: [], publishers: [], owners: ["Tersoal", "Laura", "Hugo"], locations: ["Salón", "Estudio"] };
    case "lan_status":
      return lan;
    case "sync_with":
      return { received: { received: 0, known: 0, rejected: 0, skippedChanges: 0 }, sent: 0 };
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
    case "get_settings":
      return { googleBooksApiKey: null };
    default:
      return null;
  }
});
