import { type SubmitEvent, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { CircleAlert, CircleCheck, CircleHelp, Copy as CopyIcon, Loader2 } from "lucide-react";

import { Field } from "@/components/book-form";
import { Suggestions, useCatalogOptions } from "@/components/catalog-options";
import { Cover } from "@/components/cover";
import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  type CopyInput,
  type Format,
  type LookupResult,
  type NewBook,
  createBook,
  formats,
  isCommandError,
  lookupIsbn,
} from "@/lib/api";

type Status =
  "queued" | "working" | "added" | "copyAdded" | "exists" | "notFound" | "invalid" | "failed";

interface Entry {
  id: number;
  isbn: string;
  status: Status;
  title?: string;
  coverUrl?: string | null;
  /** Copy details as they were when the ISBN was entered. */
  copy: CopyInput;
  /** Kept for "exists" and "notFound", which can still be saved by hand. */
  lookup?: LookupResult;
  error?: string;
}

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Called after every saved book, so the catalog refreshes behind the dialog. */
  onChanged: () => void;
}

function bookFrom(found: LookupResult, copy: CopyInput, title?: string): NewBook {
  return {
    title: title ?? found.title ?? "",
    subtitle: found.subtitle,
    authors: found.authors,
    publisher: found.publisher,
    publishedYear: found.publishedYear,
    language: found.language,
    isbn: found.isbn13,
    coverUrl: found.coverUrl,
    googleBooksId: found.googleBooksId,
    openLibraryId: found.openLibraryId,
    copy,
  };
}

export function BurstDialog({ open, onOpenChange, onChanged }: Props) {
  const { t } = useTranslation();
  const [owner, setOwner] = useState("");
  const [format, setFormat] = useState<Format>("paperback");
  const [location, setLocation] = useState("");
  const [isbn, setIsbn] = useState("");
  const [entries, setEntries] = useState<Entry[]>([]);
  const options = useCatalogOptions();

  // The queue lives in refs so fast scanning never waits for a render.
  const queue = useRef<Entry[]>([]);
  const running = useRef(false);
  const nextId = useRef(1);

  const sessionCopy = (): CopyInput => ({
    owner: owner.trim() || null,
    format,
    quantity: 1,
    location: location.trim() || null,
    acquiredOn: null,
  });

  const update = (id: number, patch: Partial<Entry>) =>
    setEntries((all) => all.map((e) => (e.id === id ? { ...e, ...patch } : e)));

  const process = async (entry: Entry) => {
    update(entry.id, { status: "working" });
    try {
      const found = await lookupIsbn(entry.isbn);
      const base = { title: found.title ?? undefined, coverUrl: found.coverUrl, lookup: found };
      if (found.existingEditionId) {
        update(entry.id, { ...base, status: "exists" });
      } else if (!found.found || !found.title) {
        update(entry.id, { ...base, status: "notFound" });
      } else {
        await createBook(bookFrom(found, entry.copy));
        update(entry.id, { ...base, status: "added" });
        onChanged();
      }
    } catch (e) {
      const invalid = isCommandError(e) && e.kind === "validation";
      update(entry.id, {
        status: invalid ? "invalid" : "failed",
        error: invalid
          ? t(`errors.${e.errors[0]?.code}`)
          : isCommandError(e) && e.kind === "internal"
            ? e.message
            : String(e),
      });
    }
  };

  const drain = async () => {
    if (running.current) return;
    running.current = true;
    for (let entry = queue.current.shift(); entry; entry = queue.current.shift()) {
      await process(entry);
    }
    running.current = false;
  };

  const enqueue = (event: SubmitEvent) => {
    event.preventDefault();
    const value = isbn.trim();
    if (!value) return;
    const entry: Entry = {
      id: nextId.current++,
      isbn: value,
      status: "queued",
      copy: sessionCopy(),
    };
    setEntries((all) => [entry, ...all]);
    queue.current.push(entry);
    setIsbn("");
    void drain();
  };

  const saveAnyway = async (entry: Entry, title?: string) => {
    if (!entry.lookup) return;
    update(entry.id, { status: "working" });
    try {
      await createBook(bookFrom(entry.lookup, entry.copy, title));
      update(entry.id, {
        status: entry.status === "exists" ? "copyAdded" : "added",
        title: title ?? entry.title,
      });
      onChanged();
    } catch (e) {
      update(entry.id, { status: "failed", error: String(e) });
    }
  };

  const count = (...statuses: Status[]) =>
    entries.filter((e) => statuses.includes(e.status)).length;

  const handleOpenChange = (next: boolean) => {
    if (!next) {
      setEntries([]);
      queue.current = [];
    }
    onOpenChange(next);
  };

  return (
    <ResponsiveDialog
      open={open}
      onOpenChange={handleOpenChange}
      title={t("burst.title")}
      description={t("burst.description")}
      className="sm:max-w-2xl"
    >
      <div className="grid gap-3 sm:grid-cols-3">
        <Field id="burst-owner" label={t("form.fields.owner")}>
          <Input
            id="burst-owner"
            list="burst-known-owners"
            value={owner}
            onChange={(e) => setOwner(e.target.value)}
          />
          <Suggestions id="burst-known-owners" values={options.owners} />
        </Field>
        <Field id="burst-format" label={t("form.fields.format")}>
          <Select value={format} onValueChange={(v) => setFormat(v as Format)}>
            <SelectTrigger id="burst-format" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {formats.map((f) => (
                <SelectItem key={f} value={f}>
                  {t(`formats.${f}`)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>
        <Field id="burst-location" label={t("form.fields.location")}>
          <Input
            id="burst-location"
            list="burst-known-locations"
            value={location}
            onChange={(e) => setLocation(e.target.value)}
          />
          <Suggestions id="burst-known-locations" values={options.locations} />
        </Field>
      </div>

      <form onSubmit={enqueue} className="flex gap-2">
        <Input
          autoFocus
          inputMode="numeric"
          aria-label={t("form.fields.isbn")}
          placeholder={t("lookup.placeholder")}
          value={isbn}
          onChange={(e) => setIsbn(e.target.value)}
          className="text-lg"
        />
        <Button type="submit">{t("burst.add")}</Button>
      </form>

      {entries.length > 0 && (
        <p className="text-sm text-muted-foreground">
          {t("burst.summary", {
            added: count("added", "copyAdded"),
            exists: count("exists"),
            pending: count("notFound", "invalid", "failed"),
          })}
        </p>
      )}

      <ul className="-mx-2 flex-1 space-y-2 overflow-y-auto px-2">
        {entries.map((entry) => (
          <EntryRow
            key={entry.id}
            entry={entry}
            onSaveAnyway={(title) => void saveAnyway(entry, title)}
          />
        ))}
      </ul>
    </ResponsiveDialog>
  );
}

function EntryRow({
  entry,
  onSaveAnyway,
}: {
  entry: Entry;
  onSaveAnyway: (title?: string) => void;
}) {
  const { t } = useTranslation();
  const [title, setTitle] = useState("");

  const icon = {
    queued: <Loader2 className="size-4 text-muted-foreground" />,
    working: <Loader2 className="size-4 animate-spin" />,
    added: <CircleCheck className="size-4 text-green-600" />,
    copyAdded: <CircleCheck className="size-4 text-green-600" />,
    exists: <CopyIcon className="size-4 text-amber-600" />,
    notFound: <CircleHelp className="size-4 text-amber-600" />,
    invalid: <CircleAlert className="size-4 text-destructive" />,
    failed: <CircleAlert className="size-4 text-destructive" />,
  }[entry.status];

  return (
    <li className="flex items-start gap-3 rounded-lg border p-2">
      <Cover url={entry.coverUrl} alt="" className="h-12" />
      <div className="min-w-0 flex-1 space-y-1 text-sm">
        <div className="flex items-center gap-2">
          {icon}
          <span className="truncate font-medium">{entry.title ?? entry.isbn}</span>
        </div>
        <p className="text-muted-foreground">
          {entry.isbn} · {t(`burst.status.${entry.status}`)}
          {entry.error ? ` ${entry.error}` : ""}
        </p>
        {entry.status === "exists" && (
          <Button size="sm" variant="outline" onClick={() => onSaveAnyway()}>
            {t("burst.addCopyAnyway")}
          </Button>
        )}
        {entry.status === "notFound" && (
          <form
            className="flex gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              if (title.trim()) onSaveAnyway(title.trim());
            }}
          >
            <Input
              aria-label={t("form.fields.title")}
              placeholder={t("burst.titlePlaceholder")}
              value={title}
              onChange={(e) => setTitle(e.target.value)}
            />
            <Button type="submit" size="sm" disabled={!title.trim()}>
              {t("form.save")}
            </Button>
          </form>
        )}
      </div>
    </li>
  );
}
