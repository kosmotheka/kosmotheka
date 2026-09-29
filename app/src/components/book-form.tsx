import { type ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Cloud, ImagePlus, Info, Loader2, MapPin, Plus, Search, X } from "lucide-react";

import { Suggestions, useCatalogOptions } from "@/components/catalog-options";
import { Cover } from "@/components/cover";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger } from "@/components/ui/select";
import {
  type BookDetail,
  type ContributorRole,
  type CopyDetail,
  type CopyInput,
  type EditionInput,
  type FieldError,
  type Format,
  type Kind,
  type LookupResult,
  contributorRoles,
  isCommandError,
  kinds,
  lookupIsbn,
} from "@/lib/api";
import {
  DIGITAL_FORMATS,
  DIGITAL_PLATFORMS,
  KIND_FORMATS,
  KIND_ICONS,
  VIDEOGAME_PLATFORMS,
} from "@/lib/kinds";
import { cn } from "@/lib/utils";

// Form state keeps every value as typed text; conversion happens on submit.

export interface EditionForm {
  kind: Kind;
  title: string;
  subtitle: string;
  /** Every contributor by role; `author` is the book's authors. */
  people: Partial<Record<ContributorRole, string[]>>;
  publisher: string;
  publishedYear: string;
  language: string;
  isbn: string;
  /** EAN/UPC, for what has no ISBN. */
  barcode: string;
  /** Comic issue number. */
  number: string;
  playersMin: string;
  playersMax: string;
  /** Board game or film minutes. */
  durationMinutes: string;
  minAge: string;
  /** Total seasons of a TV series (not which ones a copy holds, see `CopyForm.seasons`). */
  seasons: string;
  notes: string;
  /** Cover found online, downloaded when the form is saved. */
  coverUrl: string;
  /** Cover already stored (file name), shown when there is no new one. */
  cover: string | null;
  googleBooksId: string | null;
  openLibraryId: string | null;
}

export interface CopyForm {
  owner: string;
  format: Format;
  quantity: string;
  location: string;
  acquiredOn: string;
  /** Video game copy's platform. */
  platform: string;
  /** TV series copy's held seasons, free text ("1-2"). */
  seasons: string;
}

export const emptyEditionForm: EditionForm = {
  kind: "book",
  title: "",
  subtitle: "",
  people: {},
  publisher: "",
  publishedYear: "",
  language: "",
  isbn: "",
  barcode: "",
  number: "",
  playersMin: "",
  playersMax: "",
  durationMinutes: "",
  minAge: "",
  seasons: "",
  notes: "",
  coverUrl: "",
  cover: null,
  googleBooksId: null,
  openLibraryId: null,
};

export const emptyCopyForm: CopyForm = {
  owner: "",
  format: "paperback",
  quantity: "1",
  location: "",
  acquiredOn: "",
  platform: "",
  seasons: "",
};

/** A blank copy with the first format that kind offers, instead of a book default. */
export function emptyCopyFormFor(kind: Kind): CopyForm {
  return { ...emptyCopyForm, format: KIND_FORMATS[kind][0] };
}

const orNull = (value: string) => value.trim() || null;
const numberOrNull = (value: string) => (value.trim() === "" ? null : Number(value));

export function toEditionInput(form: EditionForm): EditionInput {
  return {
    kind: form.kind,
    title: form.title,
    subtitle: orNull(form.subtitle),
    // Every person goes through `contributors`, authors included as role "author" (D68-ish).
    authors: [],
    contributors: contributorRoles.flatMap((role) =>
      (form.people[role] ?? []).map((name) => ({ name, role })),
    ),
    publisher: orNull(form.publisher),
    publishedYear: numberOrNull(form.publishedYear),
    language: orNull(form.language),
    isbn: orNull(form.isbn),
    barcode: orNull(form.barcode),
    number: orNull(form.number),
    playersMin: numberOrNull(form.playersMin),
    playersMax: numberOrNull(form.playersMax),
    durationMinutes: numberOrNull(form.durationMinutes),
    minAge: numberOrNull(form.minAge),
    seasons: numberOrNull(form.seasons),
    notes: orNull(form.notes),
    coverUrl: orNull(form.coverUrl),
    googleBooksId: form.googleBooksId,
    openLibraryId: form.openLibraryId,
  };
}

export function toCopyInput(form: CopyForm): CopyInput {
  return {
    owner: orNull(form.owner),
    format: form.format,
    quantity: numberOrNull(form.quantity) ?? 1,
    location: orNull(form.location),
    acquiredOn: orNull(form.acquiredOn),
    platform: orNull(form.platform),
    seasons: orNull(form.seasons),
  };
}

export function editionFormFrom(book: BookDetail): EditionForm {
  const people: Partial<Record<ContributorRole, string[]>> = {};
  if (book.authors.length > 0) people.author = book.authors;
  for (const contributor of book.contributors) {
    (people[contributor.role] ??= []).push(contributor.name);
  }
  return {
    kind: book.kind,
    title: book.title,
    subtitle: book.subtitle ?? "",
    people,
    publisher: book.publisher ?? "",
    publishedYear: book.publishedYear?.toString() ?? "",
    language: book.language ?? "",
    isbn: book.isbn13 ?? "",
    barcode: book.barcode ?? "",
    number: book.number ?? "",
    playersMin: book.playersMin?.toString() ?? "",
    playersMax: book.playersMax?.toString() ?? "",
    durationMinutes: book.durationMinutes?.toString() ?? "",
    minAge: book.minAge?.toString() ?? "",
    seasons: book.seasons?.toString() ?? "",
    notes: book.notes ?? "",
    coverUrl: "",
    cover: book.cover,
    googleBooksId: book.googleBooksId,
    openLibraryId: book.openLibraryId,
  };
}

/** Fills the form with what was found, keeping what the user typed where nothing was. */
export function applyLookup(form: EditionForm, found: LookupResult): EditionForm {
  const pick = (value: string | number | null, current: string) =>
    value === null || value === "" ? current : String(value);
  return {
    ...form,
    isbn: found.isbn13,
    title: pick(found.title, form.title),
    subtitle: pick(found.subtitle, form.subtitle),
    people: {
      ...form.people,
      author: found.authors.length > 0 ? found.authors : form.people.author,
    },
    publisher: pick(found.publisher, form.publisher),
    publishedYear: pick(found.publishedYear, form.publishedYear),
    language: pick(found.language, form.language),
    coverUrl: found.coverUrl ?? form.coverUrl,
    googleBooksId: found.googleBooksId ?? form.googleBooksId,
    openLibraryId: found.openLibraryId ?? form.openLibraryId,
  };
}

export function copyFormFrom(copy: CopyDetail): CopyForm {
  return {
    owner: copy.owner ?? "",
    format: copy.format,
    quantity: copy.quantity.toString(),
    location: copy.location ?? "",
    acquiredOn: copy.acquiredOn ?? "",
    platform: copy.platform ?? "",
    seasons: copy.seasons ?? "",
  };
}

/** Field errors and unexpected failures of a command, ready for the form. */
export function useCommandErrors() {
  const { t } = useTranslation();
  const [fieldErrors, setFieldErrors] = useState<FieldError[]>([]);
  const [message, setMessage] = useState<string | null>(null);

  return {
    message,
    errorFor: (field: string) => {
      const error = fieldErrors.find((e) => e.field === field);
      return error ? t(`errors.${error.code}`) : null;
    },
    clear: () => {
      setFieldErrors([]);
      setMessage(null);
    },
    handle: (error: unknown) => {
      if (!isCommandError(error)) {
        setMessage(String(error));
      } else if (error.kind === "validation") {
        setFieldErrors(error.errors);
      } else if (error.kind === "backup" || error.kind === "sync") {
        setMessage(error.reason);
      } else if (error.kind === "permission") {
        setMessage(t(`people.errors.${error.reason}`));
      } else {
        setMessage(error.message);
      }
    },
  };
}

type ErrorFor = (field: string) => string | null;

export function Field({
  id,
  label,
  error,
  required,
  children,
}: {
  id: string;
  label: string;
  error?: string | null;
  required?: boolean;
  children: ReactNode;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>
        {label}
        {required && (
          <span aria-hidden className="-ml-1 text-muted-foreground">
            *
          </span>
        )}
      </Label>
      {children}
      {error && <p className="text-sm text-destructive">{error}</p>}
    </div>
  );
}

/** A row of tiles to pick the kind: 4x2 on mobile, 8x1 on desktop (Design's TypeSheet/FormDialog). */
export function KindGrid({
  value,
  onChange,
  columns,
  size = "sm",
}: {
  value: Kind;
  onChange: (kind: Kind) => void;
  columns: number;
  size?: "sm" | "lg";
}) {
  const { t } = useTranslation();
  return (
    <div
      role="radiogroup"
      aria-label={t("form.fields.kind")}
      className="grid gap-1.5"
      style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}
    >
      {kinds.map((k) => {
        const Icon = KIND_ICONS[k];
        const active = k === value;
        return (
          <button
            key={k}
            type="button"
            role="radio"
            aria-checked={active}
            onClick={() => onChange(k)}
            className={cn(
              "flex flex-col items-center justify-center gap-1.5 rounded-xl px-1 text-center text-xs leading-tight font-medium",
              size === "lg" ? "h-[76px]" : "h-[60px]",
              active
                ? "bg-accent text-foreground ring-2 ring-foreground ring-inset"
                : "text-muted-foreground ring-1 ring-border ring-inset",
            )}
          >
            <Icon className={size === "lg" ? "size-[22px]" : "size-[18px]"} />
            {t(`types.${k}`)}
          </button>
        );
      })}
    </div>
  );
}

/** A chip input for one contributor role: existing chips plus a text field that adds on Enter/comma. */
function PeopleField({
  id,
  label,
  values,
  onChange,
  suggestions,
}: {
  id: string;
  label: string;
  values: string[];
  onChange: (values: string[]) => void;
  suggestions: string[];
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState("");

  const add = (name: string) => {
    const trimmed = name.trim();
    if (trimmed && !values.includes(trimmed)) onChange([...values, trimmed]);
    setDraft("");
  };

  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <div className="flex min-h-11 flex-wrap items-center gap-1.5 rounded-lg border border-input px-2 py-1.5">
        {values.map((name) => (
          <span
            key={name}
            className="flex h-[30px] items-center gap-1 rounded-md bg-secondary py-0 pr-1 pl-2.5 text-sm text-secondary-foreground"
          >
            {name}
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="size-6"
              aria-label={t("form.people.remove", { name })}
              onClick={() => onChange(values.filter((v) => v !== name))}
            >
              <X className="size-3.5" />
            </Button>
          </span>
        ))}
        <input
          id={id}
          list={`${id}-suggestions`}
          className="min-w-24 flex-1 bg-transparent text-[15px] outline-none placeholder:text-muted-foreground"
          placeholder={t("form.people.add")}
          value={draft}
          onChange={(e) => {
            const typed = e.target.value;
            if (typed.includes(",")) {
              typed.split(",").forEach(add);
            } else if (suggestions.includes(typed)) {
              // Picking a suggestion fires the same input event as typing it: add right away.
              add(typed);
            } else {
              setDraft(typed);
            }
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              add(draft);
            } else if (e.key === "Backspace" && draft === "" && values.length > 0) {
              onChange(values.slice(0, -1));
            }
          }}
        />
      </div>
      <Suggestions
        id={`${id}-suggestions`}
        values={suggestions.filter((s) => !values.includes(s))}
      />
    </div>
  );
}

export function EditionFields({
  value,
  onChange,
  errorFor,
  currentEditionId,
  showKindSelect = true,
  onKindChange,
}: {
  value: EditionForm;
  onChange: (value: EditionForm) => void;
  errorFor: ErrorFor;
  /** When editing, finding this same edition by ISBN is not "already in the catalog". */
  currentEditionId?: string;
  /** Desktop's add form shows a tile row outside this component instead (`KindGrid`). */
  showKindSelect?: boolean;
  /**
   * Editing an existing work: picking another kind opens the change-of-kind
   * confirmation instead of switching right away. Omitted in the add form, where
   * there is nothing to lose yet.
   */
  onKindChange?: (kind: Kind) => void;
}) {
  const { t } = useTranslation();
  const set = <K extends keyof EditionForm>(key: K, v: EditionForm[K]) =>
    onChange({ ...value, [key]: v });
  const setPeople = (role: ContributorRole, names: string[]) =>
    onChange({ ...value, people: { ...value.people, [role]: names } });
  const lookup = useIsbnLookup();
  const options = useCatalogOptions();
  const KindIcon = KIND_ICONS[value.kind];

  const search = async () => {
    const found = await lookup.run(value.isbn);
    if (found?.found) onChange(applyLookup(value, found));
  };

  const barcodeField = (hint: string) => (
    <Field id="barcode" label={t("form.fields.barcode")} error={errorFor("barcode")}>
      <Input
        id="barcode"
        inputMode="numeric"
        value={value.barcode}
        onChange={(e) => set("barcode", e.target.value)}
      />
      <p className="text-xs text-muted-foreground">{hint}</p>
    </Field>
  );

  const durationField = (id: "durationMinutes") => (
    <Field id={id} label={t("form.fields.durationMinutes")} error={errorFor(id)}>
      <div className="flex items-center gap-2">
        <Input
          id={id}
          type="number"
          inputMode="numeric"
          value={value[id]}
          onChange={(e) => set(id, e.target.value)}
        />
        <span className="text-sm whitespace-nowrap text-muted-foreground">
          {t("form.units.minutes")}
        </span>
      </div>
    </Field>
  );

  const publisherField = (labelKey: string) => (
    <Field id="publisher" label={t(labelKey)}>
      <Input
        id="publisher"
        list="publisher-suggestions"
        value={value.publisher}
        onChange={(e) => set("publisher", e.target.value)}
      />
      <Suggestions id="publisher-suggestions" values={options.publishers} />
    </Field>
  );

  const people = (role: ContributorRole, labelKey: string) => (
    <PeopleField
      id={`people-${role}`}
      label={t(labelKey)}
      values={value.people[role] ?? []}
      onChange={(names) => setPeople(role, names)}
      suggestions={options.authors}
    />
  );

  let kindFields: ReactNode;
  switch (value.kind) {
    case "book":
      kindFields = (
        <>
          {people("author", "form.people.author")}
          <div className="grid grid-cols-[2fr_1fr] gap-4">
            {publisherField("form.fields.publisher")}
            <Field id="language" label={t("form.fields.language")}>
              <Input
                id="language"
                placeholder={t("form.placeholders.language")}
                value={value.language}
                onChange={(e) => set("language", e.target.value)}
              />
            </Field>
          </div>
          <div className="space-y-2">
            <Label htmlFor="isbn">{t("form.fields.isbn")}</Label>
            <div className="flex gap-2">
              <Input
                id="isbn"
                inputMode="numeric"
                placeholder={t("lookup.placeholder")}
                value={value.isbn}
                onChange={(e) => set("isbn", e.target.value)}
                onKeyDown={(e) => {
                  // Enter searches instead of submitting: this is how barcode scanners type.
                  if (e.key === "Enter") {
                    e.preventDefault();
                    void search();
                  }
                }}
              />
              <Button
                type="button"
                variant="secondary"
                disabled={lookup.loading || value.isbn.trim() === ""}
                onClick={() => void search()}
              >
                {lookup.loading ? <Loader2 className="animate-spin" /> : <Search />}
                {t("lookup.search")}
              </Button>
            </div>
            {errorFor("isbn") && <p className="text-sm text-destructive">{errorFor("isbn")}</p>}
            <p className="text-xs text-muted-foreground">{t("form.hints.isbn")}</p>
            <LookupStatus lookup={lookup} currentEditionId={currentEditionId} />
          </div>
        </>
      );
      break;
    case "comic":
      kindFields = (
        <>
          {people("writer", "form.people.writer")}
          {people("illustrator", "form.people.illustrator")}
          <div className="grid grid-cols-[2fr_1fr] gap-4">
            {publisherField("form.fields.publisher")}
            <Field id="number" label={t("form.fields.number")}>
              <Input
                id="number"
                value={value.number}
                onChange={(e) => set("number", e.target.value)}
              />
            </Field>
          </div>
          {barcodeField(t("form.hints.barcode"))}
        </>
      );
      break;
    case "boardgame":
      kindFields = (
        <>
          {people("designer", "form.people.designer")}
          {publisherField("form.fields.publisher")}
          <div className="grid grid-cols-3 gap-3">
            <div className="space-y-2">
              <Label htmlFor="playersMin">{t("form.fields.players")}</Label>
              <div className="flex items-center gap-2">
                <Input
                  id="playersMin"
                  aria-label={t("form.fields.playersMin")}
                  type="number"
                  inputMode="numeric"
                  value={value.playersMin}
                  onChange={(e) => set("playersMin", e.target.value)}
                />
                <span aria-hidden className="text-muted-foreground">
                  –
                </span>
                <Input
                  aria-label={t("form.fields.playersMax")}
                  type="number"
                  inputMode="numeric"
                  value={value.playersMax}
                  onChange={(e) => set("playersMax", e.target.value)}
                />
              </div>
              {(errorFor("playersMin") ?? errorFor("playersMax")) && (
                <p className="text-sm text-destructive">
                  {errorFor("playersMin") ?? errorFor("playersMax")}
                </p>
              )}
            </div>
            {durationField("durationMinutes")}
            <Field id="minAge" label={t("form.fields.minAge")} error={errorFor("minAge")}>
              <div className="flex items-center gap-2">
                <Input
                  id="minAge"
                  type="number"
                  inputMode="numeric"
                  value={value.minAge}
                  onChange={(e) => set("minAge", e.target.value)}
                />
                <span className="text-sm whitespace-nowrap text-muted-foreground">
                  {t("form.units.years")}
                </span>
              </div>
            </Field>
          </div>
          {barcodeField(t("form.hints.barcode"))}
        </>
      );
      break;
    case "film":
      kindFields = (
        <>
          {people("director", "form.people.director")}
          <div className="grid grid-cols-[1fr_2fr] gap-4">
            {durationField("durationMinutes")}
            {barcodeField(t("form.hints.barcode"))}
          </div>
        </>
      );
      break;
    case "tv_series":
      kindFields = (
        <div className="grid grid-cols-[1fr_2fr] gap-4">
          <Field id="seasons" label={t("form.fields.seasons")} error={errorFor("seasons")}>
            <Input
              id="seasons"
              type="number"
              inputMode="numeric"
              value={value.seasons}
              onChange={(e) => set("seasons", e.target.value)}
            />
          </Field>
          {barcodeField(t("form.hints.barcode"))}
        </div>
      );
      break;
    case "videogame":
      kindFields = (
        <>
          {people("studio", "form.people.studio")}
          {barcodeField(t("form.hints.barcodeVideogame"))}
        </>
      );
      break;
    case "music":
      kindFields = (
        <>
          {people("artist", "form.people.artist")}
          {/* Design pairs Sello with the barcode only on desktop; mobile stacks them. */}
          <div className="grid gap-4 sm:grid-cols-2">
            {publisherField("form.fields.label")}
            {barcodeField(t("form.hints.barcode"))}
          </div>
        </>
      );
      break;
    case "other":
      kindFields = barcodeField(t("form.hints.barcode"));
      break;
  }

  return (
    <div className="space-y-4">
      {showKindSelect && (
        <Field id="kind" label={t("form.fields.kind")}>
          <Select
            value={value.kind}
            onValueChange={(v) => (onKindChange ? onKindChange(v as Kind) : set("kind", v as Kind))}
          >
            <SelectTrigger id="kind" className="w-full justify-start">
              <span className="flex flex-1 items-center gap-2 overflow-hidden">
                <KindIcon className="size-4 shrink-0 text-muted-foreground" />
                <span className="truncate">{t(`types.${value.kind}`)}</span>
              </span>
            </SelectTrigger>
            <SelectContent position="popper">
              {kinds.map((k) => {
                const Icon = KIND_ICONS[k];
                return (
                  <SelectItem key={k} value={k}>
                    <Icon className="size-4" /> {t(`types.${k}`)}
                  </SelectItem>
                );
              })}
            </SelectContent>
          </Select>
        </Field>
      )}
      <div className="flex gap-4">
        <div className="relative shrink-0">
          <Cover
            file={value.cover}
            url={value.coverUrl || null}
            alt={value.title}
            kind={value.kind}
            className="h-36"
          />
          <Button
            type="button"
            variant="secondary"
            size="icon-sm"
            disabled
            aria-label={t("form.fields.changeCover")}
            className="absolute right-1 bottom-1 rounded-full shadow"
          >
            <ImagePlus className="size-4" />
          </Button>
        </div>
        <div className="flex-1 space-y-3">
          <Field id="title" label={t("form.fields.title")} error={errorFor("title")} required>
            <Input
              id="title"
              aria-required
              autoFocus
              value={value.title}
              onChange={(e) => set("title", e.target.value)}
            />
          </Field>
          <Field
            id="publishedYear"
            label={t("form.fields.publishedYear")}
            error={errorFor("publishedYear")}
          >
            <Input
              id="publishedYear"
              type="number"
              inputMode="numeric"
              value={value.publishedYear}
              onChange={(e) => set("publishedYear", e.target.value)}
            />
          </Field>
        </div>
      </div>
      {value.kind === "book" && (
        <Field id="subtitle" label={t("form.fields.subtitle")}>
          <Input
            id="subtitle"
            value={value.subtitle}
            onChange={(e) => set("subtitle", e.target.value)}
          />
        </Field>
      )}
      <div className="flex items-center gap-2.5 pt-1">
        <span className="text-sm font-semibold">{t(`form.kindHeading.${value.kind}`)}</span>
        <span className="h-px flex-1 bg-border" />
      </div>
      {kindFields}
      {value.kind === "other" && (
        <div className="flex gap-2.5 rounded-xl bg-muted p-3 text-[13px] leading-snug text-pretty text-muted-foreground">
          <Info className="mt-0.5 size-4 shrink-0" />
          <span>{t("form.other.notice")}</span>
        </div>
      )}
      <Field id="notes" label={t("form.fields.notes")}>
        <textarea
          id="notes"
          rows={3}
          className="w-full rounded-lg border border-input bg-transparent px-2.5 py-2 text-[15px] outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
          value={value.notes}
          onChange={(e) => set("notes", e.target.value)}
        />
      </Field>
    </div>
  );
}

export function CopyFields({
  value,
  onChange,
  errorFor,
  kind,
  idPrefix = "copy",
}: {
  value: CopyForm;
  onChange: (value: CopyForm) => void;
  errorFor: ErrorFor;
  /** Which formats and platform suggestions apply. */
  kind: Kind;
  /** Keeps element ids unique when several copy forms exist on screen. */
  idPrefix?: string;
}) {
  const { t } = useTranslation();
  const set = <K extends keyof CopyForm>(key: K, v: CopyForm[K]) =>
    onChange({ ...value, [key]: v });
  const id = (name: string) => `${idPrefix}-${name}`;
  const options = useCatalogOptions();
  const [customPlatform, setCustomPlatform] = useState(false);

  const kindFormats = KIND_FORMATS[kind];
  const digital = DIGITAL_FORMATS.has(value.format);
  const locationSuggestions = digital ? DIGITAL_PLATFORMS[kind] : options.locations;
  const platforms =
    value.platform && !VIDEOGAME_PLATFORMS.includes(value.platform)
      ? [...VIDEOGAME_PLATFORMS, value.platform]
      : VIDEOGAME_PLATFORMS;

  return (
    <div className="space-y-4">
      {kind === "videogame" && (
        <div className="space-y-2">
          <Label>{t("form.fields.platform")}</Label>
          <div
            role="radiogroup"
            aria-label={t("form.fields.platform")}
            className="flex flex-wrap gap-1.5"
          >
            {platforms.map((p) => {
              const active = value.platform === p && !customPlatform;
              return (
                <button
                  key={p}
                  type="button"
                  role="radio"
                  aria-checked={active}
                  onClick={() => {
                    setCustomPlatform(false);
                    set("platform", p);
                  }}
                  className={cn(
                    "flex h-9 items-center gap-1.5 rounded-lg px-3.5 text-sm font-medium",
                    active
                      ? "bg-primary text-primary-foreground"
                      : "text-foreground ring-1 ring-border ring-inset",
                  )}
                >
                  {active && <Check className="size-3.5" strokeWidth={2.5} />}
                  {p}
                </button>
              );
            })}
            <button
              type="button"
              role="radio"
              aria-checked={customPlatform}
              onClick={() => setCustomPlatform(true)}
              className={cn(
                "flex h-9 items-center gap-1.5 rounded-lg border border-dashed px-3 text-sm text-muted-foreground",
                customPlatform && "border-solid border-input text-foreground",
              )}
            >
              <Plus className="size-3.5" /> {t("form.fields.platformOther")}
            </button>
          </div>
          {customPlatform && (
            <Input
              aria-label={t("form.fields.platform")}
              autoFocus
              value={value.platform}
              onChange={(e) => set("platform", e.target.value)}
            />
          )}
          {errorFor("platform") && (
            <p className="text-sm text-destructive">{errorFor("platform")}</p>
          )}
        </div>
      )}
      <div className="space-y-2">
        <Label>{t("form.fields.format")}</Label>
        {kindFormats.length === 1 ? (
          <p className="text-sm text-muted-foreground">{t("form.otherFormatOnly")}</p>
        ) : (
          <div
            role="radiogroup"
            aria-label={t("form.fields.format")}
            className="flex flex-wrap gap-1.5"
          >
            {kindFormats.map((format) => {
              const active = value.format === format;
              return (
                <button
                  key={format}
                  type="button"
                  role="radio"
                  aria-checked={active}
                  onClick={() => set("format", format)}
                  className={cn(
                    "flex h-9 items-center gap-1.5 rounded-lg px-3.5 text-sm font-medium",
                    active
                      ? "bg-primary text-primary-foreground"
                      : "text-foreground ring-1 ring-border ring-inset",
                  )}
                >
                  {active && <Check className="size-3.5" strokeWidth={2.5} />}
                  {t(`formats.${format}`)}
                  {DIGITAL_FORMATS.has(format) && (
                    <Cloud
                      className={cn("size-3.5", active ? "opacity-80" : "text-muted-foreground")}
                      aria-hidden
                    />
                  )}
                </button>
              );
            })}
          </div>
        )}
      </div>
      {kind === "tv_series" && (
        <Field id={id("seasons")} label={t("form.fields.seasonsHeld")} error={errorFor("seasons")}>
          <Input
            id={id("seasons")}
            placeholder={t("form.hints.seasonsHeld")}
            value={value.seasons}
            onChange={(e) => set("seasons", e.target.value)}
          />
        </Field>
      )}
      <div className="grid gap-4 sm:grid-cols-2">
        <Field id={id("owner")} label={t("form.fields.owner")}>
          <Input
            id={id("owner")}
            list={id("known-owners")}
            value={value.owner}
            onChange={(e) => set("owner", e.target.value)}
          />
          <Suggestions id={id("known-owners")} values={options.owners} />
        </Field>
        <Field id={id("quantity")} label={t("form.fields.quantity")} error={errorFor("quantity")}>
          <Input
            id={id("quantity")}
            type="number"
            min={1}
            inputMode="numeric"
            value={value.quantity}
            onChange={(e) => set("quantity", e.target.value)}
          />
        </Field>
        <Field
          id={id("acquiredOn")}
          label={t("form.fields.acquiredOn")}
          error={errorFor("acquiredOn")}
        >
          <Input
            id={id("acquiredOn")}
            type="date"
            value={value.acquiredOn}
            onChange={(e) => set("acquiredOn", e.target.value)}
          />
        </Field>
      </div>
      <Field id={id("location")} label={t("form.fields.location")}>
        <div className="flex items-center gap-2">
          {digital ? (
            <Cloud className="size-4 shrink-0 text-muted-foreground" aria-hidden />
          ) : (
            <MapPin className="size-4 shrink-0 text-muted-foreground" aria-hidden />
          )}
          <Input
            id={id("location")}
            list={id("known-locations")}
            className="flex-1"
            placeholder={digital ? undefined : t("form.placeholders.location")}
            value={value.location}
            onChange={(e) => set("location", e.target.value)}
          />
        </div>
        <Suggestions id={id("known-locations")} values={locationSuggestions} />
        {digital && (
          <p className="text-xs text-muted-foreground">{t("form.hints.locationDigital")}</p>
        )}
      </Field>
    </div>
  );
}

type Lookup = ReturnType<typeof useIsbnLookup>;

export function useIsbnLookup() {
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<LookupResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = async (isbn: string): Promise<LookupResult | null> => {
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const found = await lookupIsbn(isbn);
      setResult(found);
      return found;
    } catch (e) {
      setError(
        isCommandError(e) && e.kind === "validation"
          ? `errors.${e.errors[0]?.code}`
          : "lookup.failed",
      );
      return null;
    } finally {
      setLoading(false);
    }
  };

  return { loading, result, error, run };
}

export function LookupStatus({
  lookup,
  currentEditionId,
}: {
  lookup: Lookup;
  currentEditionId?: string;
}) {
  const { t } = useTranslation();
  const { result, error } = lookup;
  if (error) return <p className="text-sm text-destructive">{t(error)}</p>;
  if (!result) return null;

  const quota = result.warnings.some((w) => w.source === "google_books" && w.kind === "quota");
  const unreachable = result.warnings.filter((w) => w.kind !== "quota");
  const alreadyHere =
    result.existingEditionId !== null && result.existingEditionId !== currentEditionId;

  return (
    <div className="space-y-1 text-sm">
      {result.found ? (
        <p className="text-muted-foreground">
          {t("lookup.found", {
            sources: result.sources.map((s) => t(`lookup.sources.${s}`)).join(", "),
          })}
        </p>
      ) : (
        <p className="text-muted-foreground">{t("lookup.notFound")}</p>
      )}
      {alreadyHere && (
        <p className="font-medium text-amber-600 dark:text-amber-400">
          {t("lookup.alreadyInCatalog")}
        </p>
      )}
      {quota && <p className="text-muted-foreground">{t("lookup.googleQuota")}</p>}
      {unreachable.map((w) => (
        <p key={w.source} className="text-muted-foreground">
          {t("lookup.unreachable", { source: t(`lookup.sources.${w.source}`) })}
        </p>
      ))}
    </div>
  );
}
