import { type SubmitEvent, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, X } from "lucide-react";

import { NoteField } from "@/components/ask-dialog";
import { ConfirmButton } from "@/components/confirm-button";
import {
  type CopyForm,
  type EditionForm,
  CopyFields,
  EditionFields,
  KindGrid,
  applyLookup,
  emptyCopyFormFor,
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
import { useBackGuard } from "@/lib/navigation";
import {
  type ContributorRole,
  type CreatedBook,
  type Kind,
  type LookupResult,
  type NewBook,
  addMissing,
  createBook,
  requestBook,
  requestMissing,
} from "@/lib/api";
import { KIND_FORMATS } from "@/lib/kinds";

export interface AddBookPrefill {
  isbn?: string;
  title?: string;
  author?: string;
  /** Everything a lookup already found (such as «¿Lo tengo?»), cover included. */
  found?: LookupResult;
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
  /** Shown as a back arrow instead of the close button: returns to the search step (D60). */
  onBack?: () => void;
  /** «Me falta» (D61): no copy section; saves with `addMissing` (readers: `requestMissing`). */
  missing?: boolean;
}

const initialEditionForm = (prefill?: AddBookPrefill, initial?: NewBook): EditionForm =>
  initial ? editionFormFromInput(initial) : prefillForm(prefill);

function prefillForm(prefill?: AddBookPrefill): EditionForm {
  const form: EditionForm = {
    ...emptyEditionForm,
    isbn: prefill?.isbn ?? "",
    title: prefill?.title ?? "",
    people: prefill?.author ? { author: [prefill.author] } : {},
  };
  return prefill?.found ? applyLookup(form, prefill.found) : form;
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
  onBack,
  missing = false,
}: Props) {
  const { t } = useTranslation();
  // Readers ask for the book instead of adding it (D73).
  const asking = useRequests().myRole === "reader" && !customSubmit;
  const [note, setNote] = useState("");
  const [edition, setEdition] = useState(() => initialEditionForm(prefill, initial));
  const [copy, setCopy] = useState(() =>
    initial ? copyFormFromInput(initial) : emptyCopyFormFor(edition.kind),
  );
  const [saving, setSaving] = useState(false);
  const errors = useCommandErrors();
  // A fresh add (no ISBN prefill, no request to correct) starts by asking "What is it?"
  // on mobile; desktop shows the type row inline in the form instead.
  const [pickingKind, setPickingKind] = useState(!initial && !prefill);

  // The chosen format may not exist for the new kind (e.g. "Pocket" picked, then switched
  // to a kind without it): fall back to that kind's first format.
  const changeEdition = (next: EditionForm) => {
    if (next.kind !== edition.kind && !KIND_FORMATS[next.kind].includes(copy.format)) {
      setCopy((c) => ({ ...c, format: KIND_FORMATS[next.kind][0] }));
    }
    setEdition(next);
  };

  // What was typed, against what the form opened with: back or close only ask when they differ.
  const typed = JSON.stringify([edition, copy, note.trim()]);
  const [start, setStart] = useState(typed);
  const dirty = typed !== start;
  /** Where the person was going when asked whether to discard: closing, or back to the search. */
  const [discarding, setDiscarding] = useState<"close" | "search" | null>(null);
  const leaving = onBack ? "search" : "close";

  // Back with something typed asks before losing it, instead of closing the form.
  const release = useBackGuard(open && dirty, () => {
    setDiscarding((asked) => (asked ? null : leaving));
    return true;
  });

  const handleOpenChange = (next: boolean) => {
    if (!next) {
      setEdition(emptyEditionForm);
      setCopy(emptyCopyFormFor(emptyEditionForm.kind));
      setNote("");
      // Emptied, it is not «something typed»: no back entry while the overlay closes.
      setStart(JSON.stringify([emptyEditionForm, emptyCopyFormFor(emptyEditionForm.kind), ""]));
      setPickingKind(!initial && !prefill);
      errors.clear();
    }
    onOpenChange(next);
  };

  const leave = (to: "close" | "search") => {
    if (to === "search") onBack?.();
    else handleOpenChange(false);
  };
  const askToLeave = (to: "close" | "search") => {
    if (dirty) setDiscarding(to);
    else leave(to);
  };
  const discard = async () => {
    const to = discarding;
    setDiscarding(null);
    await release();
    if (to) leave(to);
  };

  const submit = async (event: SubmitEvent) => {
    event.preventDefault();
    setSaving(true);
    errors.clear();
    try {
      if (missing) {
        const input = toEditionInput(edition);
        if (asking) {
          const { requested } = await requestMissing(input, note.trim() || null);
          await release();
          onRequested?.(requested);
        } else {
          const result = await addMissing(input);
          await release();
          onCreated(result, edition.title.trim());
        }
      } else {
        const book = { ...toEditionInput(edition), copy: toCopyInput(copy) };
        if (asking) {
          const { requested } = await requestBook(book, note.trim() || null);
          await release();
          onRequested?.(requested);
        } else {
          const result = await (customSubmit ?? createBook)(book);
          await release();
          onCreated(result, edition.title.trim());
        }
      }
      handleOpenChange(false);
    } catch (error) {
      errors.handle(error);
    } finally {
      setSaving(false);
    }
  };

  const pickKind = (kind: Kind) => changeEdition({ ...edition, kind });

  const fields = (
    <>
      <EditionFields
        value={edition}
        onChange={changeEdition}
        errorFor={errors.errorFor}
        showKindSelect={!desktop}
      />
      {missing ? (
        <p className="border-t pt-5 text-[13px] leading-snug text-pretty text-muted-foreground">
          {t("form.missingHint")}
        </p>
      ) : (
        <section className="space-y-4 border-t pt-5">
          <h3 className="text-sm font-medium">{t("form.copySection")}</h3>
          <CopyFields
            value={copy}
            onChange={setCopy}
            errorFor={(field) => errors.errorFor(`copy.${field}`)}
            kind={edition.kind}
          />
        </section>
      )}
      {asking && (
        <section className="space-y-3 border-t pt-5">
          <NoteField value={note} onChange={setNote} />
          <p className="text-[13px] leading-snug text-pretty text-muted-foreground">
            {t(missing ? "ask.whyAddMissing" : "ask.whyAdd", { library })}
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
    customTitle ??
    (missing
      ? t(asking ? "ask.addMissingTitle" : "form.missingTitle", { library })
      : t(asking ? "ask.addTitle" : "form.title", { library }));
  const submitLabel = saving ? t("form.saving") : asking ? t("ask.send") : t("form.save");

  if (!desktop) {
    return (
      <Dialog
        open={open}
        onOpenChange={(next) => (next ? handleOpenChange(true) : askToLeave("close"))}
      >
        <ScreenDialogContent>
          <div className="relative flex min-h-0 flex-1 flex-col">
            <form
              onSubmit={(e) => void submit(e)}
              noValidate
              className="flex min-h-0 flex-1 flex-col"
            >
              <header className="flex h-16 shrink-0 items-center gap-1 px-1">
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-lg"
                  className="size-12 rounded-full"
                  aria-label={onBack ? t("addSearch.backToSearch") : t("common.close")}
                  onClick={() => askToLeave(leaving)}
                >
                  {onBack ? <ArrowLeft className="size-[22px]" /> : <X className="size-[22px]" />}
                </Button>
                <DialogTitle className="flex-1 text-lg font-semibold">{title}</DialogTitle>
                <Button type="submit" className="mr-3 rounded-full px-4" disabled={saving}>
                  {submitLabel}
                </Button>
              </header>
              <DialogDescription className="sr-only">{t("form.description")}</DialogDescription>
              <div className="flex-1 space-y-6 overflow-y-auto px-4 pt-2 pb-[calc(2.5rem+env(safe-area-inset-bottom))]">
                {fields}
              </div>
            </form>
            {pickingKind && (
              <div className="absolute inset-0 z-10 flex flex-col justify-end bg-black/10">
                <div className="flex flex-col gap-3.5 rounded-t-[28px] bg-popover px-4 pt-2.5 pb-[max(1.75rem,env(safe-area-inset-bottom))]">
                  <div
                    className="mx-auto h-1 w-8 rounded-full bg-muted-foreground/40"
                    aria-hidden
                  />
                  <div className="flex flex-col gap-1">
                    <span className="text-lg font-semibold">{t("form.whatIsIt")}</span>
                    <span className="text-[13.5px] leading-snug text-pretty text-muted-foreground">
                      {t("form.whatIsItHint")}
                    </span>
                  </div>
                  <KindGrid
                    value={edition.kind}
                    onChange={(kind) => {
                      // Choosing what it is starts the form; it is not a change to discard.
                      const nextCopy = KIND_FORMATS[kind].includes(copy.format)
                        ? copy
                        : { ...copy, format: KIND_FORMATS[kind][0] };
                      setEdition({ ...edition, kind });
                      setCopy(nextCopy);
                      setStart(JSON.stringify([{ ...edition, kind }, nextCopy, note.trim()]));
                      setPickingKind(false);
                    }}
                    columns={4}
                    size="lg"
                  />
                </div>
              </div>
            )}
          </div>
        </ScreenDialogContent>
        <ConfirmButton
          open={discarding !== null}
          onOpenChange={(asked) => !asked && setDiscarding(null)}
          title={t("book.discard.title")}
          description={t("book.discard.body")}
          confirmLabel={t("book.discard.confirm")}
          onConfirm={() => void discard()}
        />
      </Dialog>
    );
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => (next ? handleOpenChange(true) : askToLeave("close"))}
    >
      {/* The body scrolls; the title and the buttons stay in sight. */}
      <DialogContent className="flex max-h-[90vh] flex-col overflow-hidden sm:max-w-2xl">
        <form
          onSubmit={(e) => void submit(e)}
          noValidate
          className="flex min-h-0 flex-1 flex-col gap-4"
        >
          <DialogHeader className="shrink-0">
            {onBack && (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="-ml-2 w-fit gap-1.5 text-muted-foreground"
                onClick={() => askToLeave("search")}
              >
                <ArrowLeft className="size-4" /> {t("addSearch.backToSearch")}
              </Button>
            )}
            <DialogTitle>{title}</DialogTitle>
            <DialogDescription className="sr-only">{t("form.description")}</DialogDescription>
          </DialogHeader>
          <div className="-mx-4 min-h-0 flex-1 space-y-6 overflow-y-auto px-4 py-1">
            <div className="space-y-2">
              <p className="text-xs font-medium text-muted-foreground">{t("form.whatIsIt")}</p>
              <KindGrid value={edition.kind} onChange={pickKind} columns={8} />
            </div>
            {fields}
          </div>
          <DialogFooter className="shrink-0">
            <Button type="button" variant="outline" onClick={() => askToLeave("close")}>
              {t("form.cancel")}
            </Button>
            <Button type="submit" disabled={saving}>
              {submitLabel}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
      <ConfirmButton
        open={discarding !== null}
        onOpenChange={(asked) => !asked && setDiscarding(null)}
        title={t("book.discard.title")}
        description={t("book.discard.body")}
        confirmLabel={t("book.discard.confirm")}
        onConfirm={() => void discard()}
      />
    </Dialog>
  );
}

function editionFormFromInput(book: NewBook): EditionForm {
  const people: Partial<Record<ContributorRole, string[]>> = {};
  if (book.authors.length > 0) people.author = book.authors;
  for (const contributor of book.contributors ?? []) {
    (people[contributor.role] ??= []).push(contributor.name);
  }
  return {
    kind: book.kind ?? "book",
    title: book.title,
    subtitle: book.subtitle ?? "",
    people,
    publisher: book.publisher ?? "",
    publishedYear: book.publishedYear?.toString() ?? "",
    language: book.language ?? "",
    isbn: book.isbn ?? "",
    barcode: book.barcode ?? "",
    number: book.number ?? "",
    playersMin: book.playersMin?.toString() ?? "",
    playersMax: book.playersMax?.toString() ?? "",
    durationMinutes: book.durationMinutes?.toString() ?? "",
    minAge: book.minAge?.toString() ?? "",
    seasons: book.seasons?.toString() ?? "",
    genres: book.genres ?? [],
    tags: book.tags ?? [],
    notes: book.notes ?? "",
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
    platform: book.copy.platform ?? "",
    seasons: book.copy.seasons ?? "",
  };
}
