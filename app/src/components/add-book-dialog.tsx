import { type FormEvent, useState } from "react";
import { useTranslation } from "react-i18next";
import { X } from "lucide-react";

import { NoteField } from "@/components/ask-dialog";
import {
  type CopyForm,
  type EditionForm,
  CopyFields,
  EditionFields,
  emptyCopyForm,
  emptyEditionForm,
  toCopyInput,
  toEditionInput,
  useCommandErrors,
} from "@/components/book-form";
import { useRequests } from "@/components/requests-context";
import { ScreenDialogContent } from "@/components/shell/screen-dialog";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { type CreatedBook, type NewBook, createBook, requestBook } from "@/lib/api";

export interface AddBookPrefill {
  isbn?: string;
  title?: string;
  author?: string;
}

interface Props {
  open: boolean;
  desktop: boolean;
  library: string;
  onOpenChange: (open: boolean) => void;
  onCreated: (result: CreatedBook, title: string) => void;
  /** Read once when the dialog mounts; give it a new `key` to prefill again. */
  prefill?: AddBookPrefill;
  /** A whole book to start from, such as a request to correct before accepting it. */
  initial?: NewBook;
  /** Saves the book another way than creating it; the title changes with it. */
  submit?: (book: NewBook) => Promise<CreatedBook>;
  title?: string;
  /** A reader's book went to the administrators, or was simply added if allowed now. */
  onRequested?: (requested: boolean) => void;
}

export function AddBookDialog({
  open,
  desktop,
  library,
  onOpenChange,
  onCreated,
  prefill,
  initial,
  submit: customSubmit,
  title: customTitle,
  onRequested,
}: Props) {
  const { t } = useTranslation();
  // Readers ask for the book instead of adding it (D73).
  const asking = useRequests().myRole === "reader" && !customSubmit;
  const [note, setNote] = useState("");
  const [edition, setEdition] = useState(() =>
    initial
      ? editionFormFromInput(initial)
      : {
          ...emptyEditionForm,
          isbn: prefill?.isbn ?? "",
          title: prefill?.title ?? "",
          authors: prefill?.author ? [prefill.author] : emptyEditionForm.authors,
        },
  );
  const [copy, setCopy] = useState(() => (initial ? copyFormFromInput(initial) : emptyCopyForm));
  const [saving, setSaving] = useState(false);
  const errors = useCommandErrors();

  const handleOpenChange = (next: boolean) => {
    if (!next) {
      setEdition(emptyEditionForm);
      setCopy(emptyCopyForm);
      setNote("");
      errors.clear();
    }
    onOpenChange(next);
  };

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setSaving(true);
    errors.clear();
    try {
      const book = { ...toEditionInput(edition), copy: toCopyInput(copy) };
      if (asking) {
        const { requested } = await requestBook(book, note.trim() || null);
        onRequested?.(requested);
      } else {
        const result = await (customSubmit ?? createBook)(book);
        onCreated(result, edition.title.trim());
      }
      handleOpenChange(false);
    } catch (error) {
      errors.handle(error);
    } finally {
      setSaving(false);
    }
  };

  const fields = (
    <>
      <EditionFields value={edition} onChange={setEdition} errorFor={errors.errorFor} />
      <section className="space-y-4 border-t pt-5">
        <h3 className="text-sm font-medium">{t("form.copySection")}</h3>
        <CopyFields
          value={copy}
          onChange={setCopy}
          errorFor={(field) => errors.errorFor(`copy.${field}`)}
        />
      </section>
      {asking && (
        <section className="space-y-3 border-t pt-5">
          <NoteField value={note} onChange={setNote} />
          <p className="text-[13px] leading-snug text-pretty text-muted-foreground">
            {t("ask.whyAdd", { library })}
          </p>
        </section>
      )}
      {errors.message && (
        <p className="text-sm text-destructive">
          {t("errors.internal", { message: errors.message })}
        </p>
      )}
    </>
  );
  const title =
    customTitle ?? (asking ? t("ask.addTitle", { library }) : t("form.title", { library }));
  const submitLabel = saving ? t("form.saving") : asking ? t("ask.send") : t("form.save");

  if (!desktop) {
    return (
      <Dialog open={open} onOpenChange={handleOpenChange}>
        <ScreenDialogContent>
          <form onSubmit={submit} noValidate className="flex min-h-0 flex-1 flex-col">
            <header className="flex h-16 shrink-0 items-center gap-1 px-1">
              <Button
                type="button"
                variant="ghost"
                size="icon-lg"
                className="size-12 rounded-full"
                aria-label={t("common.close")}
                onClick={() => handleOpenChange(false)}
              >
                <X className="size-[22px]" />
              </Button>
              <DialogTitle className="flex-1 text-lg font-semibold">{title}</DialogTitle>
              <Button type="submit" className="mr-3 rounded-full px-4" disabled={saving}>
                {submitLabel}
              </Button>
            </header>
            <DialogDescription className="sr-only">{t("form.description")}</DialogDescription>
            <div className="flex-1 space-y-6 overflow-y-auto px-4 pt-2 pb-10">{fields}</div>
          </form>
        </ScreenDialogContent>
      </Dialog>
    );
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <form onSubmit={submit} noValidate className="space-y-6">
          <DialogHeader>
            <DialogTitle>{title}</DialogTitle>
            <DialogDescription className="sr-only">{t("form.description")}</DialogDescription>
          </DialogHeader>
          {fields}
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => handleOpenChange(false)}>
              {t("form.cancel")}
            </Button>
            <Button type="submit" disabled={saving}>
              {submitLabel}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function editionFormFromInput(book: NewBook): EditionForm {
  return {
    title: book.title,
    subtitle: book.subtitle ?? "",
    authors: book.authors.length > 0 ? book.authors : [""],
    publisher: book.publisher ?? "",
    publishedYear: book.publishedYear?.toString() ?? "",
    language: book.language ?? "",
    isbn: book.isbn ?? "",
    coverUrl: book.coverUrl ?? "",
    cover: null,
    googleBooksId: book.googleBooksId,
    openLibraryId: book.openLibraryId,
  };
}

function copyFormFromInput(book: NewBook): CopyForm {
  return {
    owner: book.copy.owner ?? "",
    format: book.copy.format,
    quantity: book.copy.quantity.toString(),
    location: book.copy.location ?? "",
    acquiredOn: book.copy.acquiredOn ?? "",
  };
}
