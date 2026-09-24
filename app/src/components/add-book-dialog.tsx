import { type FormEvent, useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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
  type CreatedBook,
  type FieldError,
  type Format,
  createBook,
  formats,
  isCommandError,
} from "@/lib/api";

interface FormState {
  title: string;
  subtitle: string;
  authors: string[];
  publisher: string;
  publishedYear: string;
  language: string;
  isbn: string;
  owner: string;
  format: Format;
  quantity: string;
  location: string;
  acquiredOn: string;
}

const emptyForm: FormState = {
  title: "",
  subtitle: "",
  authors: [""],
  publisher: "",
  publishedYear: "",
  language: "",
  isbn: "",
  owner: "",
  format: "paperback",
  quantity: "1",
  location: "",
  acquiredOn: "",
};

const orNull = (value: string) => value.trim() || null;
const numberOrNull = (value: string) =>
  value.trim() === "" ? null : Number(value);

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  knownOwners: string[];
  onCreated: (result: CreatedBook, title: string) => void;
}

export function AddBookDialog({
  open,
  onOpenChange,
  knownOwners,
  onCreated,
}: Props) {
  const { t } = useTranslation();
  const [form, setForm] = useState<FormState>(emptyForm);
  const [errors, setErrors] = useState<FieldError[]>([]);
  const [internalError, setInternalError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const set = <K extends keyof FormState>(key: K, value: FormState[K]) =>
    setForm((current) => ({ ...current, [key]: value }));

  const errorFor = (field: string) => {
    const error = errors.find((e) => e.field === field);
    return error ? t(`errors.${error.code}`) : null;
  };

  const handleOpenChange = (next: boolean) => {
    if (!next) {
      setForm(emptyForm);
      setErrors([]);
      setInternalError(null);
    }
    onOpenChange(next);
  };

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setSaving(true);
    setErrors([]);
    setInternalError(null);
    try {
      const result = await createBook({
        title: form.title,
        subtitle: orNull(form.subtitle),
        authors: form.authors,
        publisher: orNull(form.publisher),
        publishedYear: numberOrNull(form.publishedYear),
        language: orNull(form.language),
        isbn: orNull(form.isbn),
        copy: {
          owner: orNull(form.owner),
          format: form.format,
          quantity: numberOrNull(form.quantity) ?? 1,
          location: orNull(form.location),
          acquiredOn: orNull(form.acquiredOn),
        },
      });
      onCreated(result, form.title.trim());
      handleOpenChange(false);
    } catch (error) {
      if (!isCommandError(error)) {
        setInternalError(String(error));
      } else if (error.kind === "validation") {
        setErrors(error.errors);
      } else {
        setInternalError(error.message);
      }
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
            <Field id="title" label={t("form.fields.title")} error={errorFor("title")}>
              <Input
                id="title"
                autoFocus
                value={form.title}
                onChange={(e) => set("title", e.target.value)}
              />
            </Field>
            <Field id="subtitle" label={t("form.fields.subtitle")}>
              <Input
                id="subtitle"
                value={form.subtitle}
                onChange={(e) => set("subtitle", e.target.value)}
              />
            </Field>
            <div className="space-y-2">
              <Label>{t("form.fields.authors")}</Label>
              {form.authors.map((author, index) => (
                <div key={index} className="flex gap-2">
                  <Input
                    aria-label={t("form.fields.author", { number: index + 1 })}
                    value={author}
                    onChange={(e) =>
                      set(
                        "authors",
                        form.authors.map((a, i) =>
                          i === index ? e.target.value : a,
                        ),
                      )
                    }
                  />
                  {form.authors.length > 1 && (
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      aria-label={t("form.removeAuthor")}
                      onClick={() =>
                        set(
                          "authors",
                          form.authors.filter((_, i) => i !== index),
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
                onClick={() => set("authors", [...form.authors, ""])}
              >
                <Plus /> {t("form.addAuthor")}
              </Button>
            </div>
            <div className="grid gap-4 sm:grid-cols-2">
              <Field id="publisher" label={t("form.fields.publisher")}>
                <Input
                  id="publisher"
                  value={form.publisher}
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
                  value={form.publishedYear}
                  onChange={(e) => set("publishedYear", e.target.value)}
                />
              </Field>
              <Field id="isbn" label={t("form.fields.isbn")} error={errorFor("isbn")}>
                <Input
                  id="isbn"
                  inputMode="numeric"
                  value={form.isbn}
                  onChange={(e) => set("isbn", e.target.value)}
                />
              </Field>
              <Field id="language" label={t("form.fields.language")}>
                <Input
                  id="language"
                  placeholder={t("form.placeholders.language")}
                  value={form.language}
                  onChange={(e) => set("language", e.target.value)}
                />
              </Field>
            </div>
          </section>

          <section className="space-y-4">
            <h3 className="text-sm font-medium">{t("form.copySection")}</h3>
            <div className="grid gap-4 sm:grid-cols-2">
              <Field id="owner" label={t("form.fields.owner")}>
                <Input
                  id="owner"
                  list="known-owners"
                  value={form.owner}
                  onChange={(e) => set("owner", e.target.value)}
                />
                <datalist id="known-owners">
                  {knownOwners.map((owner) => (
                    <option key={owner} value={owner} />
                  ))}
                </datalist>
              </Field>
              <Field id="format" label={t("form.fields.format")}>
                <Select
                  value={form.format}
                  onValueChange={(value) => set("format", value as Format)}
                >
                  <SelectTrigger id="format" className="w-full">
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
                id="quantity"
                label={t("form.fields.quantity")}
                error={errorFor("copy.quantity")}
              >
                <Input
                  id="quantity"
                  type="number"
                  min={1}
                  inputMode="numeric"
                  value={form.quantity}
                  onChange={(e) => set("quantity", e.target.value)}
                />
              </Field>
              <Field
                id="acquiredOn"
                label={t("form.fields.acquiredOn")}
                error={errorFor("copy.acquiredOn")}
              >
                <Input
                  id="acquiredOn"
                  type="date"
                  value={form.acquiredOn}
                  onChange={(e) => set("acquiredOn", e.target.value)}
                />
              </Field>
            </div>
            <Field id="location" label={t("form.fields.location")}>
              <Input
                id="location"
                placeholder={t("form.placeholders.location")}
                value={form.location}
                onChange={(e) => set("location", e.target.value)}
              />
            </Field>
          </section>

          {internalError && (
            <p className="text-sm text-destructive">
              {t("errors.internal", { message: internalError })}
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

function Field({
  id,
  label,
  error,
  children,
}: {
  id: string;
  label: string;
  error?: string | null;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      {children}
      {error && <p className="text-sm text-destructive">{error}</p>}
    </div>
  );
}
