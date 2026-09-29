import { type FormEvent, useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import {
  ArrowLeft,
  Baby,
  Check,
  Clock,
  Ellipsis,
  EllipsisVertical,
  ExternalLink,
  FolderInput,
  Layers,
  Pencil,
  Plus,
  Trash2,
  Users,
  X,
} from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";

import {
  type CopyForm,
  CopyFields,
  EditionFields,
  type EditionForm,
  copyFormFrom,
  editionFormFrom,
  emptyCopyFormFor,
  toCopyInput,
  toEditionInput,
  useCommandErrors,
} from "@/components/book-form";
import { AskDialog } from "@/components/ask-dialog";
import { ChangeKindDialog } from "@/components/change-kind-dialog";
import { ConfirmButton } from "@/components/confirm-button";
import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { useRequests } from "@/components/requests-context";
import { Cover } from "@/components/cover";
import { HistoryList } from "@/components/history-list";
import { MarksSection } from "@/components/marks/marks-section";
import { Button } from "@/components/ui/button";
import { ScreenDialogContent } from "@/components/shell/screen-dialog";
import { useDesktop } from "@/components/shell/use-desktop";
import { Dialog, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  type BookDetail,
  type ContributorRole,
  type CopyDetail,
  type Format,
  addCopy,
  deleteBook,
  deleteCopy,
  requestDeleteBook,
  requestDeleteCopy,
  externalLinks,
  getBook,
  updateCopy,
  updateEdition,
} from "@/lib/api";
import { type KindChangePlan, planKindChange } from "@/lib/change-kind";
import { KIND_ICONS, KIND_PEOPLE, copyIcon, copyWhatText, copyWhereText } from "@/lib/kinds";
import { openOverlay } from "@/lib/navigation";
import { cn } from "@/lib/utils";

interface Props {
  editionId: string | null;
  desktop: boolean;
  onClose: () => void;
  /** Called after any change, so the catalog list can refresh. */
  onChanged: () => void;
  onNotice: (message: string) => void;
}

type Tab = "copies" | "details" | "history";

/**
 * A work's page: full screen on mobile, a panel beside the catalog on desktop.
 * Editing happens in place, in the "Details" and "Copies" tabs.
 */
export function BookDetailView({ editionId, desktop, onClose, onChanged, onNotice }: Props) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const library = useLibraryName()(libraries.active);
  // Readers change nothing themselves: they ask (D73).
  const reader = useRequests().myRole === "reader";
  const [asking, setAsking] = useState(false);
  const [loaded, setLoaded] = useState<BookDetail | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [version, setVersion] = useState(0);
  const [tab, setTab] = useState<Tab>("copies");
  const [editing, setEditing] = useState<EditionForm | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);

  const reload = useCallback(() => {
    if (!editionId) return;
    getBook(editionId)
      .then((detail) => {
        setLoaded(detail);
        setLoadError(null);
      })
      .catch((e: unknown) => setLoadError(String(e)));
  }, [editionId]);

  useEffect(reload, [reload]);

  // Another book starts on its copies, not halfway through editing the last one.
  const [shownId, setShownId] = useState(editionId);
  if (shownId !== editionId) {
    setShownId(editionId);
    setTab("copies");
    setEditing(null);
  }

  // The desktop panel is not a dialog, so Escape has to be wired by hand.
  useEffect(() => {
    if (!desktop || !editionId) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [desktop, editionId, onClose]);

  // While another book loads, show nothing rather than the previous one.
  const book = loaded?.editionId === editionId ? loaded : null;

  const changed = () => {
    setVersion((v) => v + 1);
    reload();
    onChanged();
  };

  const edit = () => {
    if (!book) return;
    setTab("details");
    setEditing(editionFormFrom(book));
  };

  const removeBook = async () => {
    if (!editionId) return;
    try {
      await deleteBook(editionId);
      onChanged();
      onClose();
    } catch (e) {
      setLoadError(String(e));
    }
  };

  const links: [string, string][] = [];
  if (book?.googleBooksId) {
    links.push([t("lookup.sources.google_books"), externalLinks.googleBooks(book.googleBooksId)]);
  }
  if (book?.openLibraryId) {
    links.push([t("lookup.sources.open_library"), externalLinks.openLibrary(book.openLibraryId)]);
  }

  const moreMenu = (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size={desktop ? "icon-sm" : "icon-lg"}
          className={desktop ? undefined : "size-12 rounded-full"}
          aria-label={t("book.more")}
        >
          {desktop ? <Ellipsis /> : <EllipsisVertical className="size-5" />}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-48">
        {links.map(([label, url]) => (
          <DropdownMenuItem key={url} onSelect={() => void openUrl(url)}>
            <ExternalLink /> {label}
          </DropdownMenuItem>
        ))}
        {links.length > 0 && <DropdownMenuSeparator />}
        {book?.canDelete && libraries.all.length > 1 && (
          <DropdownMenuItem
            onSelect={() =>
              openOverlay({ name: "move", editionId: book.editionId, title: book.title })
            }
          >
            <FolderInput /> {t("libraries.moveToAnother")}
          </DropdownMenuItem>
        )}
        {book?.canDelete !== false ? (
          <DropdownMenuItem variant="destructive" onSelect={() => setConfirmDelete(true)}>
            <Trash2 /> {t("common.delete")}
          </DropdownMenuItem>
        ) : (
          <>
            <DropdownMenuItem variant="destructive" onSelect={() => setAsking(true)}>
              <Trash2 /> {t("ask.delete")}
            </DropdownMenuItem>
            <p className="max-w-60 px-2 py-1.5 text-xs leading-snug text-muted-foreground">
              {t(reader ? "ask.whyDeleteReader" : "ask.whyDelete", { library })}
            </p>
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );

  const body = (
    <>
      {loadError && (
        <p className="px-4 text-sm text-destructive">
          {t("errors.internal", { message: loadError })}
        </p>
      )}
      {book && (
        <div className={cn("flex flex-col gap-5 pb-8", desktop ? "px-5" : "px-4")}>
          <Hero book={book} desktop={desktop} />
          <MarksSection
            key={book.workId}
            workId={book.workId}
            kind={book.kind}
            desktop={desktop}
            onNotice={onNotice}
            onChanged={onChanged}
          />
          <Tabs value={tab} onValueChange={(value) => setTab(value as Tab)}>
            <TabsList variant="line" className="w-full justify-start border-b">
              <TabsTrigger value="copies" className="flex-none">
                {t("book.tabs.copies", { count: book.copies.length })}
              </TabsTrigger>
              <TabsTrigger value="details" className="flex-none">
                {t("book.tabs.details")}
              </TabsTrigger>
              <TabsTrigger value="history" className="flex-none">
                {t("history.title")}
              </TabsTrigger>
            </TabsList>
            <TabsContent value="copies" className="pt-4">
              <CopiesSection
                book={book}
                desktop={desktop}
                reader={reader}
                onChanged={changed}
                onNotice={onNotice}
              />
            </TabsContent>
            <TabsContent value="details" className="pt-4">
              <EditionSection
                book={book}
                form={editing}
                onFormChange={setEditing}
                onSaved={changed}
              />
            </TabsContent>
            <TabsContent value="history" className="pt-4">
              <HistoryList editionId={book.editionId} version={version} />
            </TabsContent>
          </Tabs>
        </div>
      )}
      <ConfirmButton
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title={t("book.deleteConfirm.title", { title: book?.title ?? "" })}
        description={t("book.deleteConfirm.body")}
        confirmLabel={t("common.delete")}
        onConfirm={() => void removeBook()}
      />
      {book && (
        <AskDialog
          open={asking}
          desktop={desktop}
          title={t("ask.deleteTitle", { title: book.title })}
          what={t("ask.deleteBook", { title: book.title })}
          why={t(reader ? "ask.whyDeleteReader" : "ask.whyDelete", { library })}
          onClose={() => setAsking(false)}
          onSend={async (note) => {
            const { requested } = await requestDeleteBook(book.editionId, note);
            setAsking(false);
            onNotice(requested ? t("ask.sent") : t("ask.done"));
            onChanged();
            if (!requested) onClose();
          }}
        />
      )}
    </>
  );

  if (desktop) {
    if (!editionId) return null;
    return (
      <aside
        aria-label={book ? t("book.panelLabel", { title: book.title }) : undefined}
        className="flex w-[400px] shrink-0 flex-col border-l bg-background"
      >
        <div className="flex h-14 shrink-0 items-center gap-1.5 px-4">
          <span className="flex-1 text-[13px] text-muted-foreground">
            {book ? t(`types.${book.kind}`) : null}
          </span>
          {!reader && (
            <Button variant="outline" size="sm" onClick={edit} disabled={!book}>
              <Pencil /> {t("book.edit")}
            </Button>
          )}
          {moreMenu}
          <Button variant="ghost" size="icon-sm" aria-label={t("book.close")} onClick={onClose}>
            <X />
          </Button>
        </div>
        <div className="flex-1 overflow-y-auto">{body}</div>
      </aside>
    );
  }

  return (
    <Dialog open={editionId !== null} onOpenChange={(open) => !open && onClose()}>
      <ScreenDialogContent>
        <DialogTitle className="sr-only">{book?.title ?? t("catalog.loading")}</DialogTitle>
        <DialogDescription className="sr-only">
          {book?.authors.join(", ") || t("book.noAuthors")}
        </DialogDescription>
        <header className="flex h-16 shrink-0 items-center gap-1 px-1">
          <Button
            variant="ghost"
            size="icon-lg"
            className="size-12 rounded-full"
            aria-label={t("book.back")}
            onClick={onClose}
          >
            <ArrowLeft className="size-[22px]" />
          </Button>
          <span className="flex-1" />
          {!reader && (
            <Button
              variant="ghost"
              size="icon-lg"
              className="size-12 rounded-full"
              aria-label={t("book.edit")}
              onClick={edit}
              disabled={!book}
            >
              <Pencil className="size-5" />
            </Button>
          )}
          {moreMenu}
        </header>
        <div className="flex-1 overflow-y-auto">{body}</div>
      </ScreenDialogContent>
    </Dialog>
  );
}

function contributorNames(book: BookDetail, role: ContributorRole): string | null {
  const names =
    role === "author"
      ? book.authors
      : book.contributors.filter((c) => c.role === role).map((c) => c.name);
  return names.length > 0 ? names.join(", ") : null;
}

/** The header's people line: authors for a book, or the kind's main roles for the rest
 * (comic writer + illustrator, boardgame designer, film director…); none for a series or "other". */
function mainPeopleLine(book: BookDetail): string {
  return KIND_PEOPLE[book.kind]
    .map(({ role }) => contributorNames(book, role))
    .filter((name): name is string => name !== null)
    .join(", ");
}

/** The fact chips under the title, per kind (Design's WorkDetail/DetailPanel). */
function factChips(book: BookDetail, t: TFunction): { icon: typeof Users; text: string }[] {
  const chips: { icon: typeof Users; text: string }[] = [];
  if (book.kind === "boardgame") {
    if (book.playersMin && book.playersMax) {
      chips.push({ icon: Users, text: `${book.playersMin}–${book.playersMax}` });
    }
    if (book.durationMinutes) {
      chips.push({ icon: Clock, text: `${book.durationMinutes} ${t("form.units.minutes")}` });
    }
    if (book.minAge) chips.push({ icon: Baby, text: t("book.facts.minAge", { age: book.minAge }) });
  } else if (book.kind === "film" && book.durationMinutes) {
    chips.push({ icon: Clock, text: `${book.durationMinutes} ${t("form.units.minutes")}` });
  } else if (book.kind === "tv_series" && book.seasons) {
    chips.push({ icon: Layers, text: t("book.facts.seasons", { count: book.seasons }) });
  }
  return chips;
}

function Hero({ book, desktop }: { book: BookDetail; desktop: boolean }) {
  const { t } = useTranslation();
  const KindIcon = KIND_ICONS[book.kind];
  const meta = [t(`types.${book.kind}`), book.publishedYear].filter(Boolean).join(" · ");
  const by = mainPeopleLine(book);
  const chips = factChips(book, t);
  const copies = book.copies.reduce((sum, copy) => sum + copy.quantity, 0);
  return (
    <div className="flex items-end gap-4">
      <Cover
        file={book.cover}
        alt={book.title}
        kind={book.kind}
        className={cn("rounded-md", desktop ? "h-[150px]" : "h-[156px]")}
      />
      <div className="flex min-w-0 flex-col gap-1 pb-1">
        <h2 className="text-[22px] leading-tight font-semibold tracking-tight">{book.title}</h2>
        {book.subtitle && <p className="text-sm text-muted-foreground">{book.subtitle}</p>}
        {(by || book.kind === "book") && <p className="text-[15px]">{by || t("book.noAuthors")}</p>}
        <p className="flex items-center gap-1.5 text-[13px] text-muted-foreground">
          <KindIcon className="size-3.5" />
          {meta}
        </p>
        {chips.length > 0 && (
          <div className="mt-0.5 flex flex-wrap gap-1.5">
            {chips.map((chip, i) => (
              <span
                key={i}
                className="flex h-7 items-center gap-1.5 rounded-lg bg-muted px-2.5 text-[13px] font-medium"
              >
                <chip.icon className="size-3.5 text-muted-foreground" />
                {chip.text}
              </span>
            ))}
          </div>
        )}
        <div className="mt-1.5 flex flex-wrap gap-1.5">
          {copies > 0 ? (
            <span className="inline-flex items-center gap-1 rounded-full bg-success-soft px-2.5 py-0.5 text-xs font-medium text-success">
              <Check className="size-3.5" strokeWidth={2.5} />
              {t("book.owned", { count: copies })}
            </span>
          ) : (
            <span className="rounded-full bg-muted px-2.5 py-0.5 text-xs font-medium text-muted-foreground">
              {t("book.noCopies")}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

/** The "Datos" rows for a work, per kind (Design's WorkDetail/DetailPanel lists). Rows with
 * no value are skipped rather than shown as "—", except book kept its earlier row set. */
function datosRows(book: BookDetail, t: TFunction): [string, string | null][] {
  const code = book.barcode ? ([t("book.data.code"), book.barcode] as [string, string]) : null;
  const rows: [string, string | null][] = (() => {
    switch (book.kind) {
      case "book":
        return [
          [t("form.people.author"), contributorNames(book, "author")],
          [t("form.fields.publisher"), book.publisher],
          [t("form.fields.isbn"), book.isbn13],
          [t("form.fields.language"), book.language],
        ];
      case "comic":
        return [
          [t("form.people.writer"), contributorNames(book, "writer")],
          [t("form.people.illustrator"), contributorNames(book, "illustrator")],
          [t("form.fields.publisher"), book.publisher],
          [t("form.fields.number"), book.number],
          ...(code ? [code] : []),
        ];
      case "boardgame":
        return [
          [t("form.people.designer"), contributorNames(book, "designer")],
          [t("form.fields.publisher"), book.publisher],
          [
            t("form.fields.players"),
            book.playersMin && book.playersMax
              ? t("book.data.playersRange", { min: book.playersMin, max: book.playersMax })
              : null,
          ],
          [
            t("form.fields.durationMinutes"),
            book.durationMinutes ? `${book.durationMinutes} ${t("form.units.minutes")}` : null,
          ],
          [
            t("form.fields.minAge"),
            book.minAge ? t("book.data.minAgeYears", { age: book.minAge }) : null,
          ],
          ...(code ? [code] : []),
        ];
      case "film":
        return [
          [t("form.people.director"), contributorNames(book, "director")],
          [
            t("form.fields.durationMinutes"),
            book.durationMinutes ? `${book.durationMinutes} ${t("form.units.minutes")}` : null,
          ],
          ...(code ? [code] : []),
        ];
      case "tv_series":
        return [
          [
            t("form.fields.seasons"),
            book.seasons ? t("book.facts.seasons", { count: book.seasons }) : null,
          ],
          ...(code ? [code] : []),
        ];
      case "videogame":
        return [
          [t("form.people.studio"), contributorNames(book, "studio")],
          ...(code ? [code] : []),
        ];
      case "music":
        return [
          [t("form.people.artist"), contributorNames(book, "artist")],
          [t("form.fields.label"), book.publisher],
          ...(code ? [code] : []),
        ];
      case "other":
        return code ? [code] : [];
    }
  })();
  if (book.notes) rows.push([t("form.fields.notes"), book.notes]);
  return rows.filter(([, value]) => value !== null);
}

function EditionSection({
  book,
  form,
  onFormChange,
  onSaved,
}: {
  book: BookDetail;
  form: EditionForm | null;
  onFormChange: (form: EditionForm | null) => void;
  onSaved: () => void;
}) {
  const { t } = useTranslation();
  const [saving, setSaving] = useState(false);
  const [coverFailed, setCoverFailed] = useState(false);
  const [kindChangePlan, setKindChangePlan] = useState<KindChangePlan | null>(null);
  const [copyFormatChanges, setCopyFormatChanges] = useState<Record<string, Format>>({});
  const desktop = useDesktop();
  const errors = useCommandErrors();

  const save = async (event: FormEvent) => {
    event.preventDefault();
    if (!form) return;
    setSaving(true);
    errors.clear();
    try {
      const result = await updateEdition(book.editionId, toEditionInput(form));
      setCoverFailed(result.coverFailed);
      for (const copy of book.copies) {
        const newFormat = copyFormatChanges[copy.id];
        if (!newFormat || newFormat === copy.format) continue;
        try {
          await updateCopy(copy.id, { ...toCopyInput(copyFormFrom(copy)), format: newFormat });
        } catch (e) {
          errors.handle(e);
          break;
        }
      }
      setCopyFormatChanges({});
      onFormChange(null);
      onSaved();
    } catch (e) {
      errors.handle(e);
    } finally {
      setSaving(false);
    }
  };

  if (form) {
    return (
      <form onSubmit={save} noValidate className="space-y-4">
        <EditionFields
          value={form}
          onChange={onFormChange}
          errorFor={errors.errorFor}
          currentEditionId={book.editionId}
          onKindChange={(kind) => setKindChangePlan(planKindChange(book, kind))}
        />
        {errors.message && (
          <p className="text-sm text-destructive">
            {t("errors.internal", { message: errors.message })}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button
            type="button"
            variant="outline"
            onClick={() => {
              errors.clear();
              setCopyFormatChanges({});
              onFormChange(null);
            }}
          >
            {t("form.cancel")}
          </Button>
          <Button type="submit" disabled={saving}>
            {saving ? t("form.saving") : t("form.save")}
          </Button>
        </div>
        {kindChangePlan && (
          <ChangeKindDialog
            book={book}
            plan={kindChangePlan}
            desktop={desktop}
            onCancel={() => setKindChangePlan(null)}
            onConfirm={(copyFormats) => {
              onFormChange({ ...form, kind: kindChangePlan.toKind, ...kindChangePlan.formPatch });
              setCopyFormatChanges(copyFormats);
              setKindChangePlan(null);
            }}
          />
        )}
      </form>
    );
  }

  return (
    <div className="space-y-3">
      <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2.5 text-sm">
        {datosRows(book, t).map(([label, value]) => (
          <div key={label} className="contents">
            <dt className="text-muted-foreground">{label}</dt>
            <dd className="text-pretty">{value}</dd>
          </div>
        ))}
      </dl>
      {coverFailed && <p className="text-sm text-muted-foreground">{t("catalog.coverFailed")}</p>}
    </div>
  );
}

function CopiesSection({
  book,
  desktop,
  reader,
  onChanged,
  onNotice,
}: {
  book: BookDetail;
  desktop: boolean;
  reader: boolean;
  onChanged: () => void;
  onNotice: (message: string) => void;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const library = useLibraryName()(libraries.active);
  const [asking, setAsking] = useState<CopyDetail | null>(null);
  /** Id of the copy being edited, or "new" while adding one. */
  const [editing, setEditing] = useState<string | null>(null);
  const [form, setForm] = useState<CopyForm>(() => emptyCopyFormFor(book.kind));
  const [saving, setSaving] = useState(false);
  const [removing, setRemoving] = useState<CopyDetail | null>(null);
  const errors = useCommandErrors();

  const start = (target: string, initial: CopyForm) => {
    errors.clear();
    setForm(initial);
    setEditing(target);
  };

  const save = async (event: FormEvent) => {
    event.preventDefault();
    if (!editing) return;
    setSaving(true);
    errors.clear();
    try {
      if (editing === "new") {
        await addCopy(book.editionId, toCopyInput(form));
      } else {
        await updateCopy(editing, toCopyInput(form));
      }
      setEditing(null);
      onChanged();
    } catch (e) {
      errors.handle(e);
    } finally {
      setSaving(false);
    }
  };

  const remove = async (copy: CopyDetail) => {
    try {
      await deleteCopy(copy.id);
      onChanged();
    } catch (e) {
      errors.handle(e);
    }
  };

  const editor = (
    <form onSubmit={save} noValidate className="space-y-4 rounded-xl border p-4">
      <CopyFields
        value={form}
        onChange={setForm}
        errorFor={errors.errorFor}
        kind={book.kind}
        idPrefix={`copy-${editing}`}
      />
      <div className="flex justify-end gap-2">
        <Button type="button" variant="outline" onClick={() => setEditing(null)}>
          {t("form.cancel")}
        </Button>
        <Button type="submit" disabled={saving}>
          {saving ? t("form.saving") : t("form.save")}
        </Button>
      </div>
    </form>
  );

  return (
    <div className="flex flex-col gap-2">
      {book.copies.length === 0 && editing !== "new" && (
        <p className="text-sm text-muted-foreground">{t("copies.empty")}</p>
      )}
      {book.copies.map((copy) => {
        if (editing === copy.id) return <div key={copy.id}>{editor}</div>;
        const Icon = copyIcon(book.kind, copy.format);
        return (
          <div key={copy.id} className="flex items-center gap-3 rounded-xl border p-3">
            <span className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted">
              <Icon className="size-[19px]" />
            </span>
            <div className="flex min-w-0 flex-1 flex-col gap-0.5">
              <span className="text-[15px] font-medium">{copyWhatText(book.kind, copy, t)}</span>
              <span className="truncate text-[13px] text-muted-foreground">
                {copyWhereText(copy, t)}
              </span>
              {copy.acquiredOn && (
                <span className="text-[13px] text-muted-foreground">
                  {t("copies.acquired", { date: copy.acquiredOn })}
                </span>
              )}
            </div>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="rounded-full text-muted-foreground"
                  aria-label={t("copies.more")}
                >
                  <EllipsisVertical />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="min-w-48">
                {!reader && (
                  <DropdownMenuItem onSelect={() => start(copy.id, copyFormFrom(copy))}>
                    <Pencil /> {t("common.edit")}
                  </DropdownMenuItem>
                )}
                {copy.canDelete ? (
                  <DropdownMenuItem variant="destructive" onSelect={() => setRemoving(copy)}>
                    <Trash2 /> {t("common.delete")}
                  </DropdownMenuItem>
                ) : (
                  <>
                    <DropdownMenuItem variant="destructive" onSelect={() => setAsking(copy)}>
                      <Trash2 /> {t("ask.delete")}
                    </DropdownMenuItem>
                    <p className="max-w-60 px-2 py-1.5 text-xs leading-snug text-muted-foreground">
                      {t(reader ? "ask.whyDeleteReader" : "ask.whyDelete", { library })}
                    </p>
                  </>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        );
      })}
      {editing === "new" ? (
        editor
      ) : reader ? null : (
        <Button
          variant="outline"
          className="h-12 rounded-xl border-dashed"
          onClick={() => start("new", emptyCopyFormFor(book.kind))}
        >
          <Plus /> {t("copies.add")}
        </Button>
      )}
      {errors.message && (
        <p className="text-sm text-destructive">
          {t("errors.internal", { message: errors.message })}
        </p>
      )}
      <ConfirmButton
        open={removing !== null}
        onOpenChange={(open) => !open && setRemoving(null)}
        title={t("copies.deleteConfirm.title")}
        description={t("copies.deleteConfirm.body")}
        confirmLabel={t("common.delete")}
        onConfirm={() => removing && void remove(removing)}
      />
      <AskDialog
        open={asking !== null}
        desktop={desktop}
        title={t("ask.deleteCopyTitle")}
        what={
          asking
            ? t("ask.deleteCopy", { format: t(`formats.${asking.format}`), title: book.title })
            : ""
        }
        why={t(reader ? "ask.whyDeleteReader" : "ask.whyDelete", { library })}
        onClose={() => setAsking(null)}
        onSend={async (note) => {
          if (!asking) return;
          const { requested } = await requestDeleteCopy(asking.id, note);
          setAsking(null);
          onNotice(requested ? t("ask.sent") : t("ask.done"));
          onChanged();
        }}
      />
    </div>
  );
}
