import type { EditionForm } from "@/components/book-form";
import type { BookDetail, ContributorRole, Format, Kind } from "@/lib/api";
import { DIGITAL_FORMATS, KIND_FORMATS, KIND_PEOPLE } from "@/lib/kinds";

/** Kinds whose form has a publisher/label field (book-form.tsx's `publisherField`). */
const PUBLISHER_KINDS = new Set<Kind>(["book", "comic", "boardgame", "music"]);

/** Kinds whose form has a duration field. */
const DURATION_KINDS = new Set<Kind>(["boardgame", "film"]);

export type ChangeKindItem =
  | { field: "people"; role: ContributorRole; names: string[] }
  | { field: "publisher" | "language" | "isbn" | "barcode" | "number"; text: string }
  | { field: "players"; min: number | null; max: number | null }
  | { field: "duration"; minutes: number }
  | { field: "minAge"; age: number }
  | { field: "seasons"; count: number };

interface CopyKindPlan {
  copyId: string;
  from: Format;
  /** `null`: no automatic equivalent: the person must pick one. */
  to: Format | null;
}

export interface KindChangePlan {
  fromKind: Kind;
  toKind: Kind;
  /** What moves into the new kind's fields, already applied to `formPatch`. */
  kept: ChangeKindItem[];
  /** What has no equivalent in the new kind and would be dropped. */
  lost: ChangeKindItem[];
  /** Values to merge into the edit form once the kind switches. */
  formPatch: Partial<EditionForm>;
  copies: CopyKindPlan[];
}

function isValidIsbn13(code: string): boolean {
  const digits = code.replace(/\D/g, "");
  if (digits.length !== 13) return false;
  const sum = digits
    .slice(0, 12)
    .split("")
    .reduce((acc, d, i) => acc + Number(d) * (i % 2 === 0 ? 1 : 3), 0);
  return (10 - (sum % 10)) % 10 === Number(digits[12]);
}

/** The copy format an automatic equivalent maps to, or `null` when the person must pick one. */
function mapCopyFormat(from: Format, toKind: Kind): Format | null {
  const targets = KIND_FORMATS[toKind];
  if (toKind === "other") return "other";
  if (targets.includes(from)) return from;
  if (from === "ebook" || from === "pdf" || from === "audiobook") {
    return targets.includes("digital") ? "digital" : null;
  }
  if (from === "digital") {
    return targets.find((f) => DIGITAL_FORMATS.has(f)) ?? null;
  }
  if (from === "box") {
    return targets.includes("other") ? "other" : null;
  }
  if (from === "other") {
    return targets[0];
  }
  return null;
}

/**
 * What changing a work's kind would carry over, lose, and do to its copies.
 * Pure: no i18n or React here, so it stays easy to unit-test if the project ever
 * grows a TS test runner (none is set up in app/package.json at the time of writing).
 * The confirmation dialog turns `kept`/`lost` items into sentences.
 */
export function planKindChange(book: BookDetail, toKind: Kind): KindChangePlan {
  const fromKind = book.kind;
  const kept: ChangeKindItem[] = [];
  const lost: ChangeKindItem[] = [];
  const formPatch: Partial<EditionForm> = {};

  // People: every person in the old kind's roles moves into the new kind's first
  // role, if it has one. Splitting them back out by hand (e.g. a book's author into
  // a comic's writer and illustrator) is left for the person to fix after switching.
  const roleNames = new Map<ContributorRole, string[]>();
  for (const { role } of KIND_PEOPLE[fromKind]) {
    const names =
      fromKind === "book" && role === "author"
        ? book.authors
        : book.contributors.filter((c) => c.role === role).map((c) => c.name);
    if (names.length > 0) roleNames.set(role, names);
  }
  if (roleNames.size > 0) {
    const [target] = KIND_PEOPLE[toKind];
    if (target) {
      const targetRole = target.role;
      const allNames = [...roleNames.values()].flat();
      formPatch.people = { [targetRole]: allNames };
      kept.push({ field: "people", role: targetRole, names: allNames });
    } else {
      for (const [role, names] of roleNames) lost.push({ field: "people", role, names });
    }
  }

  // Publisher (or a music "label", the same field in the form).
  if (book.publisher) {
    if (PUBLISHER_KINDS.has(fromKind) && PUBLISHER_KINDS.has(toKind)) {
      formPatch.publisher = book.publisher;
      kept.push({ field: "publisher", text: book.publisher });
    } else if (PUBLISHER_KINDS.has(fromKind)) {
      lost.push({ field: "publisher", text: book.publisher });
    }
  }

  // ISBN <-> barcode: a book's ISBN becomes the new kind's barcode; leaving a
  // barcode that is a valid ISBN-13 for a book turns it into the ISBN. Otherwise
  // the barcode always stays as a barcode.
  if (fromKind === "book" && book.isbn13 && toKind !== "book") {
    formPatch.barcode = book.isbn13;
    kept.push({ field: "barcode", text: book.isbn13 });
  } else if (book.barcode) {
    if (toKind === "book" && isValidIsbn13(book.barcode)) {
      formPatch.isbn = book.barcode;
      kept.push({ field: "isbn", text: book.barcode });
    } else {
      formPatch.barcode = book.barcode;
      kept.push({ field: "barcode", text: book.barcode });
    }
  }

  // Fields with no equivalent anywhere else: always lost when leaving their kind.
  // Order follows the "Datos" rows (book-detail.tsx's `datosRows`) so the sentence
  // reads in the same order the person saw the fields in.
  if (fromKind === "comic" && book.number) {
    lost.push({ field: "number", text: book.number });
  }
  if (fromKind === "boardgame" && (book.playersMin || book.playersMax)) {
    lost.push({ field: "players", min: book.playersMin, max: book.playersMax });
  }

  // Duration is shared between boardgame and film only.
  if (book.durationMinutes && DURATION_KINDS.has(fromKind)) {
    if (DURATION_KINDS.has(toKind)) {
      formPatch.durationMinutes = String(book.durationMinutes);
      kept.push({ field: "duration", minutes: book.durationMinutes });
    } else {
      lost.push({ field: "duration", minutes: book.durationMinutes });
    }
  }

  if (fromKind === "boardgame" && book.minAge) {
    lost.push({ field: "minAge", age: book.minAge });
  }
  if (fromKind === "book" && book.language) {
    lost.push({ field: "language", text: book.language });
  }
  if (fromKind === "tv_series" && book.seasons) {
    lost.push({ field: "seasons", count: book.seasons });
  }

  const copies: CopyKindPlan[] = book.copies.map((c) => ({
    copyId: c.id,
    from: c.format,
    to: mapCopyFormat(c.format, toKind),
  }));

  return { fromKind, toKind, kept, lost, formPatch, copies };
}
