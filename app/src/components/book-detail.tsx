import { type FormEvent, useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ArrowLeft,
  Book,
  Check,
  Ellipsis,
  EllipsisVertical,
  ExternalLink,
  FolderInput,
  Headphones,
  Pencil,
  Plus,
  Tablet,
  Trash2,
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
  emptyCopyForm,
  toCopyInput,
  toEditionInput,
  useCommandErrors,
} from "@/components/book-form";
import { AskDialog } from "@/components/ask-dialog";
import { ConfirmButton } from "@/components/confirm-button";
import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { useRequests } from "@/components/requests-context";
import { Cover } from "@/components/cover";
import { HistoryList } from "@/components/history-list";
import { Button } from "@/components/ui/button";
import { ScreenDialogContent } from "@/components/shell/screen-dialog";
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
          <span className="flex-1 text-[13px] text-muted-foreground">{t("types.book")}</span>
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

function Hero({ book, desktop }: { book: BookDetail; desktop: boolean }) {
  const { t } = useTranslation();
  const meta = [t("types.book"), book.publishedYear, book.publisher].filter(Boolean).join(" · ");
  const copies = book.copies.reduce((sum, copy) => sum + copy.quantity, 0);
  return (
    <div className="flex items-end gap-4">
      <Cover
        file={book.cover}
        alt={book.title}
        className={cn("rounded-md", desktop ? "h-[150px] w-[100px]" : "h-[156px] w-[104px]")}
      />
      <div className="flex min-w-0 flex-col gap-1 pb-1">
        <h2 className="text-[22px] leading-tight font-semibold tracking-tight">{book.title}</h2>
        {book.subtitle && <p className="text-sm text-muted-foreground">{book.subtitle}</p>}
        <p className="text-[15px]">{book.authors.join(", ") || t("book.noAuthors")}</p>
        <p className="text-[13px] text-muted-foreground">{meta}</p>
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
  const errors = useCommandErrors();

  const save = async (event: FormEvent) => {
    event.preventDefault();
    if (!form) return;
    setSaving(true);
    errors.clear();
    try {
      const result = await updateEdition(book.editionId, toEditionInput(form));
      setCoverFailed(result.coverFailed);
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
              onFormChange(null);
            }}
          >
            {t("form.cancel")}
          </Button>
          <Button type="submit" disabled={saving}>
            {saving ? t("form.saving") : t("form.save")}
          </Button>
        </div>
      </form>
    );
  }

  const rows: [string, string | number | null][] = [
    [t("form.fields.isbn"), book.isbn13],
    [t("form.fields.publisher"), book.publisher],
    [t("form.fields.publishedYear"), book.publishedYear],
    [t("form.fields.language"), book.language],
  ];

  return (
    <div className="space-y-3">
      <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2.5 text-sm">
        {rows.map(([label, value]) => (
          <div key={label} className="contents">
            <dt className="text-muted-foreground">{label}</dt>
            <dd>{value ?? "—"}</dd>
          </div>
        ))}
      </dl>
      {coverFailed && <p className="text-sm text-muted-foreground">{t("catalog.coverFailed")}</p>}
    </div>
  );
}

const DIGITAL: Partial<Record<Format, typeof Book>> = { ebook: Tablet, audiobook: Headphones };

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
  const [form, setForm] = useState<CopyForm>(emptyCopyForm);
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
        const Icon = DIGITAL[copy.format] ?? Book;
        const where = [
          copy.owner ?? t("copies.noOwner"),
          copy.location,
          copy.quantity > 1 ? t("copies.units", { count: copy.quantity }) : null,
        ]
          .filter(Boolean)
          .join(" · ");
        return (
          <div key={copy.id} className="flex items-center gap-3 rounded-xl border p-3">
            <span className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted">
              <Icon className="size-[19px]" />
            </span>
            <div className="flex min-w-0 flex-1 flex-col gap-0.5">
              <span className="text-[15px] font-medium">{t(`formats.${copy.format}`)}</span>
              <span className="truncate text-[13px] text-muted-foreground">{where}</span>
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
          onClick={() => start("new", emptyCopyForm)}
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
