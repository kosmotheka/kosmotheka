import { type FormEvent, useState } from "react";
import { useTranslation } from "react-i18next";

import {
  CopyFields,
  EditionFields,
  emptyCopyForm,
  emptyEditionForm,
  toCopyInput,
  toEditionInput,
  useCommandErrors,
} from "@/components/book-form";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { type CreatedBook, createBook } from "@/lib/api";

export interface AddBookPrefill {
  isbn?: string;
  title?: string;
  author?: string;
}

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated: (result: CreatedBook, title: string) => void;
  /** Read once when the dialog mounts; give it a new `key` to prefill again. */
  prefill?: AddBookPrefill;
}

export function AddBookDialog({ open, onOpenChange, onCreated, prefill }: Props) {
  const { t } = useTranslation();
  const [edition, setEdition] = useState(() => ({
    ...emptyEditionForm,
    isbn: prefill?.isbn ?? "",
    title: prefill?.title ?? "",
    authors: prefill?.author ? [prefill.author] : emptyEditionForm.authors,
  }));
  const [copy, setCopy] = useState(emptyCopyForm);
  const [saving, setSaving] = useState(false);
  const errors = useCommandErrors();

  const handleOpenChange = (next: boolean) => {
    if (!next) {
      setEdition(emptyEditionForm);
      setCopy(emptyCopyForm);
      errors.clear();
    }
    onOpenChange(next);
  };

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setSaving(true);
    errors.clear();
    try {
      const result = await createBook({
        ...toEditionInput(edition),
        copy: toCopyInput(copy),
      });
      onCreated(result, edition.title.trim());
      handleOpenChange(false);
    } catch (error) {
      errors.handle(error);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <form onSubmit={submit} noValidate className="space-y-6">
          <DialogHeader>
            <DialogTitle>{t("form.title")}</DialogTitle>
            <DialogDescription>{t("form.description")}</DialogDescription>
          </DialogHeader>

          <section className="space-y-4">
            <h3 className="text-sm font-medium">{t("form.editionSection")}</h3>
            <EditionFields
              value={edition}
              onChange={setEdition}
              errorFor={errors.errorFor}
            />
          </section>

          <section className="space-y-4">
            <h3 className="text-sm font-medium">{t("form.copySection")}</h3>
            <CopyFields
              value={copy}
              onChange={setCopy}
              errorFor={(field) => errors.errorFor(`copy.${field}`)}
            />
          </section>

          {errors.message && (
            <p className="text-sm text-destructive">
              {t("errors.internal", { message: errors.message })}
            </p>
          )}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => handleOpenChange(false)}
            >
              {t("form.cancel")}
            </Button>
            <Button type="submit" disabled={saving}>
              {saving ? t("form.saving") : t("form.save")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
