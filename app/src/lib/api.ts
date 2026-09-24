import { invoke } from "@tauri-apps/api/core";

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

export interface NewBook {
  title: string;
  subtitle: string | null;
  authors: string[];
  publisher: string | null;
  publishedYear: number | null;
  language: string | null;
  isbn: string | null;
  copy: {
    owner: string | null;
    format: Format;
    quantity: number;
    location: string | null;
    acquiredOn: string | null;
  };
}

export interface CreatedBook {
  editionId: string;
  existingEdition: boolean;
}

export interface BookSummary {
  editionId: string;
  title: string;
  subtitle: string | null;
  authors: string[];
  publisher: string | null;
  publishedYear: number | null;
  isbn13: string | null;
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
  | "isbn_checksum";

export interface FieldError {
  field: string;
  code: FieldErrorCode;
}

export type CommandError =
  | { kind: "validation"; errors: FieldError[] }
  | { kind: "internal"; message: string };

export function isCommandError(value: unknown): value is CommandError {
  return typeof value === "object" && value !== null && "kind" in value;
}

export function createBook(book: NewBook): Promise<CreatedBook> {
  return invoke<CreatedBook>("create_book", { book });
}

export function listBooks(): Promise<BookSummary[]> {
  return invoke<BookSummary[]>("list_books");
}
