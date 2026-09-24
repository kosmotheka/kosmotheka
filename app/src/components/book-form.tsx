import { type ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";
import { Loader2, Plus, Search, X } from "lucide-react";

import { Cover } from "@/components/cover";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  type BookDetail,
  type CopyDetail,
  type CopyInput,
  type EditionInput,
  type FieldError,
  type Format,
  type LookupResult,
  formats,
  isCommandError,
  lookupIsbn,
} from "@/lib/api";

// Form state keeps every value as typed text; conversion happens on submit.

export interface EditionForm {
  title: string;
  subtitle: string;
  authors: string[];
  publisher: string;
  publishedYear: string;
  language: string;
  isbn: string;
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
}

export const emptyEditionForm: EditionForm = {
  title: "",
  subtitle: "",
  authors: [""],
  publisher: "",
  publishedYear: "",
  language: "",
  isbn: "",
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
};

const orNull = (value: string) => value.trim() || null;
const numberOrNull = (value: string) =>
  value.trim() === "" ? null : Number(value);

export function toEditionInput(form: EditionForm): EditionInput {
  return {
    title: form.title,
    subtitle: orNull(form.subtitle),
    authors: form.authors,
    publisher: orNull(form.publisher),
    publishedYear: numberOrNull(form.publishedYear),
    language: orNull(form.language),
    isbn: orNull(form.isbn),
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
  };
}

export function editionFormFrom(book: BookDetail): EditionForm {
  return {
    title: book.title,
    subtitle: book.subtitle ?? "",
    authors: book.authors.length > 0 ? book.authors : [""],
    publisher: book.publisher ?? "",
    publishedYear: book.publishedYear?.toString() ?? "",
    language: book.language ?? "",
    isbn: book.isbn13 ?? "",
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
    authors: found.authors.length > 0 ? found.authors : form.authors,
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
  children,
}: {
  id: string;
  label: string;
  error?: string | null;
  children: ReactNode;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      {children}
      {error && <p className="text-sm text-destructive">{error}</p>}
    </div>
  );
}

export function EditionFields({
  value,
  onChange,
  errorFor,
  currentEditionId,
}: {
  value: EditionForm;
  onChange: (value: EditionForm) => void;
  errorFor: ErrorFor;
  /** When editing, finding this same edition by ISBN is not "already in the catalog". */
  currentEditionId?: string;
}) {
  const { t } = useTranslation();
  const set = <K extends keyof EditionForm>(key: K, v: EditionForm[K]) =>
    onChange({ ...value, [key]: v });
  const lookup = useIsbnLookup();

  const search = async () => {
    const found = await lookup.run(value.isbn);
    if (found?.found) onChange(applyLookup(value, found));
  };

  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <Label htmlFor="isbn">{t("form.fields.isbn")}</Label>
        <div className="flex gap-2">
          <Input
            id="isbn"
            autoFocus
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
        {errorFor("isbn") && (
          <p className="text-sm text-destructive">{errorFor("isbn")}</p>
        )}
        <LookupStatus lookup={lookup} currentEditionId={currentEditionId} />
      </div>
      <div className="flex gap-4">
        <Cover
          file={value.cover}
          url={value.coverUrl || null}
          alt={value.title}
          className="h-36 w-24"
        />
        <div className="flex-1">
          <Field id="title" label={t("form.fields.title")} error={errorFor("title")}>
            <Input
              id="title"
              value={value.title}
              onChange={(e) => set("title", e.target.value)}
            />
          </Field>
        </div>
      </div>
      <Field id="subtitle" label={t("form.fields.subtitle")}>
        <Input
          id="subtitle"
          value={value.subtitle}
          onChange={(e) => set("subtitle", e.target.value)}
        />
      </Field>
      <div className="space-y-2">
        <Label>{t("form.fields.authors")}</Label>
        {value.authors.map((author, index) => (
          <div key={index} className="flex gap-2">
            <Input
              aria-label={t("form.fields.author", { number: index + 1 })}
              value={author}
              onChange={(e) =>
                set(
                  "authors",
                  value.authors.map((a, i) => (i === index ? e.target.value : a)),
                )
              }
            />
            {value.authors.length > 1 && (
              <Button
                type="button"
                variant="ghost"
                size="icon"
                aria-label={t("form.removeAuthor")}
                onClick={() =>
                  set(
                    "authors",
                    value.authors.filter((_, i) => i !== index),
                  )
                }
              >
                <X />
              </Button>
            )}
          </div>
        ))}
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => set("authors", [...value.authors, ""])}
        >
          <Plus /> {t("form.addAuthor")}
        </Button>
      </div>
      <div className="grid gap-4 sm:grid-cols-2">
        <Field id="publisher" label={t("form.fields.publisher")}>
          <Input
            id="publisher"
            value={value.publisher}
            onChange={(e) => set("publisher", e.target.value)}
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
        <Field id="language" label={t("form.fields.language")}>
          <Input
            id="language"
            placeholder={t("form.placeholders.language")}
            value={value.language}
            onChange={(e) => set("language", e.target.value)}
          />
        </Field>
      </div>
    </div>
  );
}

export function CopyFields({
  value,
  onChange,
  errorFor,
  knownOwners,
  idPrefix = "copy",
}: {
  value: CopyForm;
  onChange: (value: CopyForm) => void;
  errorFor: ErrorFor;
  knownOwners: string[];
  /** Keeps element ids unique when several copy forms exist on screen. */
  idPrefix?: string;
}) {
  const { t } = useTranslation();
  const set = <K extends keyof CopyForm>(key: K, v: CopyForm[K]) =>
    onChange({ ...value, [key]: v });
  const id = (name: string) => `${idPrefix}-${name}`;

  return (
    <div className="space-y-4">
      <div className="grid gap-4 sm:grid-cols-2">
        <Field id={id("owner")} label={t("form.fields.owner")}>
          <Input
            id={id("owner")}
            list={id("known-owners")}
            value={value.owner}
            onChange={(e) => set("owner", e.target.value)}
          />
          <datalist id={id("known-owners")}>
            {knownOwners.map((owner) => (
              <option key={owner} value={owner} />
            ))}
          </datalist>
        </Field>
        <Field id={id("format")} label={t("form.fields.format")}>
          <Select
            value={value.format}
            onValueChange={(v) => set("format", v as Format)}
          >
            <SelectTrigger id={id("format")} className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {formats.map((format) => (
                <SelectItem key={format} value={format}>
                  {t(`formats.${format}`)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>
        <Field
          id={id("quantity")}
          label={t("form.fields.quantity")}
          error={errorFor("quantity")}
        >
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
        <Input
          id={id("location")}
          placeholder={t("form.placeholders.location")}
          value={value.location}
          onChange={(e) => set("location", e.target.value)}
        />
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

  const quota = result.warnings.some(
    (w) => w.source === "google_books" && w.kind === "quota",
  );
  const unreachable = result.warnings.filter((w) => w.kind !== "quota");
  const alreadyHere =
    result.existingEditionId !== null &&
    result.existingEditionId !== currentEditionId;

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
