import type { Candidate, NewBook } from "@/lib/api";
import { KIND_FORMATS } from "@/lib/kinds";

/** The book a search result becomes, with a copy in the format the provider suggests, if the kind has it. */
export function newBookFromCandidate(candidate: Candidate): NewBook {
  const format =
    candidate.format && KIND_FORMATS[candidate.kind].includes(candidate.format)
      ? candidate.format
      : KIND_FORMATS[candidate.kind][0];
  return {
    kind: candidate.kind,
    title: candidate.title,
    subtitle: candidate.subtitle,
    authors: candidate.authors,
    contributors: candidate.contributors,
    publisher: candidate.publisher,
    publishedYear: candidate.publishedYear,
    language: candidate.language,
    isbn: candidate.isbn,
    barcode: candidate.barcode,
    number: candidate.number,
    playersMin: candidate.playersMin,
    playersMax: candidate.playersMax,
    durationMinutes: candidate.durationMinutes,
    minAge: candidate.minAge,
    seasons: candidate.seasons,
    coverUrl: candidate.coverUrl,
    googleBooksId: candidate.source === "google_books" ? candidate.externalId : null,
    openLibraryId: candidate.source === "open_library" ? candidate.externalId : null,
    copy: {
      owner: null,
      format,
      quantity: 1,
      location: null,
      acquiredOn: null,
      platform: candidate.platform,
      seasons: null,
    },
  };
}
