import { type FormEvent, useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ExternalLink, Pencil, Plus, Trash2 } from "lucide-react";
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
import { ConfirmButton } from "@/components/confirm-button";
import { Cover } from "@/components/cover";
import { HistoryList } from "@/components/history-list";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  type BookDetail,
  type CopyDetail,
  addCopy,
  deleteBook,
  deleteCopy,
  externalLinks,
  getBook,
  updateCopy,
  updateEdition,
} from "@/lib/api";

interface Props {
  editionId: string | null;
  knownOwners: string[];
  onClose: () => void;
  /** Called after any change, so the catalog list can refresh. */
  onChanged: () => void;
}

export function BookDialog({ editionId, knownOwners, onClose, onChanged }: Props) {
  const { t } = useTranslation();
  const [loaded, setLoaded] = useState<BookDetail | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [version, setVersion] = useState(0);

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

  // While another book loads, show nothing rather than the previous one.
  const book = loaded?.editionId === editionId ? loaded : null;

  const changed = () => {
    setVersion((v) => v + 1);
    reload();
    onChanged();
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

  return (
    <Dialog open={editionId !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{book?.title ?? t("catalog.loading")}</DialogTitle>
          <DialogDescription>
            {book?.authors.join(", ") || t("book.noAuthors")}
          </DialogDescription>
        </DialogHeader>
        {loadError && (
          <p className="text-sm text-destructive">
            {t("errors.internal", { message: loadError })}
          </p>
        )}
        {book && (
          <Tabs defaultValue="details">
            <TabsList>
              <TabsTrigger value="details">{t("book.tabs.details")}</TabsTrigger>
              <TabsTrigger value="copies">
                {t("book.tabs.copies", { count: book.copies.length })}
              </TabsTrigger>
              <TabsTrigger value="history">{t("history.title")}</TabsTrigger>
            </TabsList>
            <TabsContent value="details" className="pt-4">
              <EditionSection book={book} onSaved={changed} />
              <div className="mt-8 border-t pt-4">
                <ConfirmButton
                  trigger={
                    <Button variant="outline" className="text-destructive">
                      <Trash2 /> {t("book.delete")}
                    </Button>
                  }
                  title={t("book.deleteConfirm.title", { title: book.title })}
                  description={t("book.deleteConfirm.body")}
                  confirmLabel={t("book.delete")}
                  onConfirm={() => void removeBook()}
                />
              </div>
            </TabsContent>
            <TabsContent value="copies" className="pt-4">
              <CopiesSection book={book} knownOwners={knownOwners} onChanged={changed} />
            </TabsContent>
            <TabsContent value="history" className="pt-4">
              <HistoryList editionId={book.editionId} version={version} />
            </TabsContent>
          </Tabs>
        )}
      </DialogContent>
    </Dialog>
  );
}

function EditionSection({ book, onSaved }: { book: BookDetail; onSaved: () => void }) {
  const { t } = useTranslation();
  const [form, setForm] = useState<EditionForm | null>(null);
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
      setForm(null);
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
          onChange={setForm}
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
              setForm(null);
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
    [t("form.fields.subtitle"), book.subtitle],
    [t("form.fields.publisher"), book.publisher],
    [t("form.fields.publishedYear"), book.publishedYear],
    [t("form.fields.language"), book.language],
    [t("form.fields.isbn"), book.isbn13],
  ];
  const links: [string, string][] = [];
  if (book.googleBooksId) {
    links.push([t("lookup.sources.google_books"), externalLinks.googleBooks(book.googleBooksId)]);
  }
  if (book.openLibraryId) {
    links.push([t("lookup.sources.open_library"), externalLinks.openLibrary(book.openLibraryId)]);
  }

  return (
    <div className="space-y-4">
      <div className="flex gap-6">
        <Cover file={book.cover} alt={book.title} className="h-40 w-28" />
        <dl className="grid flex-1 grid-cols-[auto_1fr] content-start gap-x-6 gap-y-2 text-sm">
          {rows.map(([label, value]) => (
            <div key={label} className="contents">
              <dt className="text-muted-foreground">{label}</dt>
              <dd>{value ?? "—"}</dd>
            </div>
          ))}
        </dl>
      </div>
      {coverFailed && (
        <p className="text-sm text-muted-foreground">{t("catalog.coverFailed")}</p>
      )}
      <div className="flex flex-wrap gap-2">
        <Button variant="outline" onClick={() => setForm(editionFormFrom(book))}>
          <Pencil /> {t("book.edit")}
        </Button>
        {links.map(([label, url]) => (
          <Button key={url} variant="ghost" onClick={() => void openUrl(url)}>
            <ExternalLink /> {label}
          </Button>
        ))}
      </div>
    </div>
  );
}

function CopiesSection({
  book,
  knownOwners,
  onChanged,
}: {
  book: BookDetail;
  knownOwners: string[];
  onChanged: () => void;
}) {
  const { t } = useTranslation();
  /** Id of the copy being edited, or "new" while adding one. */
  const [editing, setEditing] = useState<string | null>(null);
  const [form, setForm] = useState<CopyForm>(emptyCopyForm);
  const [saving, setSaving] = useState(false);
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
    <form onSubmit={save} noValidate className="space-y-4 rounded-lg border p-4">
      <CopyFields
        value={form}
        onChange={setForm}
        errorFor={errors.errorFor}
        knownOwners={knownOwners}
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
    <div className="space-y-3">
      {book.copies.length === 0 && editing !== "new" && (
        <p className="text-sm text-muted-foreground">{t("copies.empty")}</p>
      )}
      {book.copies.map((copy) =>
        editing === copy.id ? (
          <div key={copy.id}>{editor}</div>
        ) : (
          <div
            key={copy.id}
            className="flex items-start justify-between gap-4 rounded-lg border p-3"
          >
            <div className="space-y-1 text-sm">
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant="secondary">{t(`formats.${copy.format}`)}</Badge>
                <span className="font-medium">
                  {copy.owner ?? t("copies.noOwner")}
                </span>
                {copy.quantity > 1 && (
                  <span className="text-muted-foreground">
                    {t("copies.units", { count: copy.quantity })}
                  </span>
                )}
              </div>
              {copy.location && (
                <p className="text-muted-foreground">{copy.location}</p>
              )}
              {copy.acquiredOn && (
                <p className="text-muted-foreground">
                  {t("copies.acquired", { date: copy.acquiredOn })}
                </p>
              )}
            </div>
            <div className="flex gap-1">
              <Button
                variant="ghost"
                size="icon"
                aria-label={t("copies.edit")}
                onClick={() => start(copy.id, copyFormFrom(copy))}
              >
                <Pencil />
              </Button>
              <ConfirmButton
                trigger={
                  <Button variant="ghost" size="icon" aria-label={t("copies.delete")}>
                    <Trash2 />
                  </Button>
                }
                title={t("copies.deleteConfirm.title")}
                description={t("copies.deleteConfirm.body")}
                confirmLabel={t("copies.delete")}
                onConfirm={() => void remove(copy)}
              />
            </div>
          </div>
        ),
      )}
      {editing === "new" ? (
        editor
      ) : (
        <Button variant="outline" onClick={() => start("new", emptyCopyForm)}>
          <Plus /> {t("copies.add")}
        </Button>
      )}
      {errors.message && (
        <p className="text-sm text-destructive">
          {t("errors.internal", { message: errors.message })}
        </p>
      )}
    </div>
  );
}
