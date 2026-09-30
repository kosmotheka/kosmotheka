import {
  Book,
  BookImage,
  Clapperboard,
  Cloud,
  Dice5,
  Disc3,
  Gamepad2,
  Headphones,
  Shapes,
  Tablet,
  TvMinimal,
  type LucideIcon,
} from "lucide-react";

import type { ContributorRole, CopyDetail, Format, Kind, MarkStatus } from "@/lib/api";

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

/** Formats with an icon of their own, instead of the cloud or the kind icon. */
const COPY_FORMAT_ICONS: Partial<Record<Format, LucideIcon>> = {
  ebook: Tablet,
  audiobook: Headphones,
};

/** A copy's icon: its format's own icon, the cloud for other digital formats, or the kind icon. */
export function copyIcon(kind: Kind, format: Format): LucideIcon {
  return COPY_FORMAT_ICONS[format] ?? (DIGITAL_FORMATS.has(format) ? Cloud : KIND_ICONS[kind]);
}

type Translate = (key: string, options?: Record<string, unknown>) => string;

/** A copy's "what" line: its format, with the video game platform or the held seasons. */
export function copyWhatText(
  kind: Kind,
  copy: Pick<CopyDetail, "format" | "platform" | "seasons">,
  t: Translate,
): string {
  const formatLabel = t(`formats.${copy.format}`);
  if (kind === "videogame" && copy.platform) return `${copy.platform} · ${formatLabel}`;
  if (kind === "tv_series" && copy.seasons) {
    const several = /[-,]/.test(copy.seasons);
    return `${formatLabel} · ${t("copies.seasonsCount", { count: several ? 2 : 1, range: copy.seasons })}`;
  }
  return formatLabel;
}

/** A copy's "where" line: who has it, where, and how many. */
export function copyWhereText(
  copy: Pick<CopyDetail, "owner" | "location" | "quantity">,
  t: Translate,
): string {
  return [
    copy.owner ?? t("copies.noOwner"),
    copy.location,
    copy.quantity > 1 ? t("copies.units", { count: copy.quantity }) : null,
  ]
    .filter(Boolean)
    .join(" · ");
}

/** Who wishes a work (D61): «Lo desea Hugo», «Lo deseas tú y Hugo», «Lo desean Laura, Hugo».
 * `wishedBy` has this device's person first, as `"me"`, when they wish it too. */
export function wishSummary(t: Translate, wishedBy: string[]): string {
  const others = wishedBy.filter((name) => name !== "me");
  if (wishedBy.includes("me")) {
    return others.length === 0
      ? t("wish.byMeOnly")
      : t("wish.byMeAnd", { names: others.join(", ") });
  }
  if (others.length === 1) return t("wish.byOne", { name: others[0] });
  if (others.length > 1) return t("wish.byMany", { names: others.join(", ") });
  return "";
}
