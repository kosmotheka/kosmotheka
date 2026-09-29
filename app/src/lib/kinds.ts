import {
  Book,
  BookImage,
  Clapperboard,
  Dice5,
  Disc3,
  Gamepad2,
  Shapes,
  TvMinimal,
  type LucideIcon,
} from "lucide-react";

import type { ContributorRole, Format, Kind, MarkStatus } from "@/lib/api";

/** Icon shown on a cover placeholder and next to the kind's name (Design, TypeReference). */
export const KIND_ICONS: Record<Kind, LucideIcon> = {
  book: Book,
  comic: BookImage,
  boardgame: Dice5,
  film: Clapperboard,
  tv_series: TvMinimal,
  videogame: Gamepad2,
  music: Disc3,
  other: Shapes,
};

/** Formats offered per kind, in the order the form and filters show them. */
export const KIND_FORMATS: Record<Kind, Format[]> = {
  book: ["hardcover", "paperback", "pocket", "ebook", "pdf", "audiobook"],
  comic: ["hardcover", "paperback", "single_issue", "digital"],
  boardgame: ["box", "digital"],
  film: ["dvd", "bluray", "uhd", "digital"],
  tv_series: ["dvd", "bluray", "digital"],
  videogame: ["physical", "digital"],
  music: ["vinyl", "cd", "cassette", "digital"],
  other: ["other"],
};

/** Formats with no shelf: the copy's "where" is a service or platform, not a location. */
export const DIGITAL_FORMATS: ReadonlySet<Format> = new Set([
  "ebook",
  "pdf",
  "audiobook",
  "digital",
]);

export type CoverShape = "tall" | "square" | "case" | "game";

/** Cover placeholder shape per kind (Design's `Cover.dc.html`). */
export const COVER_SHAPES: Record<Kind, CoverShape> = {
  book: "tall",
  comic: "tall",
  boardgame: "square",
  film: "case",
  tv_series: "case",
  videogame: "game",
  music: "square",
  other: "square",
};

/** The person/group roles the form's "own data" section asks for, per kind, in order (D68-ish). */
export const KIND_PEOPLE: Record<Kind, { role: ContributorRole; labelKey: string }[]> = {
  book: [{ role: "author", labelKey: "form.people.author" }],
  comic: [
    { role: "writer", labelKey: "form.people.writer" },
    { role: "illustrator", labelKey: "form.people.illustrator" },
  ],
  boardgame: [{ role: "designer", labelKey: "form.people.designer" }],
  film: [{ role: "director", labelKey: "form.people.director" }],
  tv_series: [],
  videogame: [{ role: "studio", labelKey: "form.people.studio" }],
  music: [{ role: "artist", labelKey: "form.people.artist" }],
  other: [],
};

/** Suggested "where" for a digital copy of each kind: services, not shelves (Design's CopySheet). */
export const DIGITAL_PLATFORMS: Record<Kind, string[]> = {
  book: ["Kindle", "Kobo", "Audible"],
  comic: ["ComiXology", "Kindle"],
  boardgame: ["Board Game Arena", "Tabletopia"],
  film: ["Apple TV", "Google TV", "Filmin"],
  tv_series: ["Apple TV", "Filmin", "Google TV"],
  videogame: ["Nintendo eShop", "Steam", "PlayStation Store"],
  music: ["Bandcamp", "Spotify", "Apple Music"],
  other: [],
};

/** Starting list of video game platforms, used until the engine offers the library's own. */
export const VIDEOGAME_PLATFORMS = ["Switch", "PS5", "PS4", "Xbox Series", "PC"];

/**
 * The i18n key for a mark status, worded for one kind ("Leyendo", "Viendo"…), or the
 * generic wording ("En curso") when `kind` is null: mixed lists, filters with no kind chip.
 */
export function statusKey(kind: Kind | null, status: MarkStatus) {
  return kind ? `marks.statusesByKind.${kind}.${status}` : `marks.statuses.${status}`;
}
