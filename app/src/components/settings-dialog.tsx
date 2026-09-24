import { type FormEvent, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Field } from "@/components/book-form";
import { DataSection } from "@/components/data-section";
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
import { getSettings, saveSettings } from "@/lib/api";

export function SettingsDialog({
  open,
  onOpenChange,
  onChanged,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Called when a restore or an import changed the catalog. */
  onChanged: () => void;
}) {
  const { t } = useTranslation();
  const [apiKey, setApiKey] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    getSettings()
      .then((settings) => setApiKey(settings.googleBooksApiKey ?? ""))
      .catch((e: unknown) => setError(String(e)));
  }, [open]);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    try {
      await saveSettings({ googleBooksApiKey: apiKey.trim() || null });
      onOpenChange(false);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto">
        <form onSubmit={submit} className="space-y-6">
          <DialogHeader>
            <DialogTitle>{t("settings.title")}</DialogTitle>
            <DialogDescription>{t("settings.description")}</DialogDescription>
          </DialogHeader>
          <Field id="google-books-key" label={t("settings.googleBooksKey")}>
            <Input
              id="google-books-key"
              type="password"
              autoComplete="off"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
            />
            <p className="text-sm text-muted-foreground">
              {t("settings.googleBooksHelp")}
            </p>
          </Field>
          {error && (
            <p className="text-sm text-destructive">
              {t("errors.internal", { message: error })}
            </p>
          )}
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              {t("form.cancel")}
            </Button>
            <Button type="submit">{t("form.save")}</Button>
          </DialogFooter>
        </form>
        <div className="border-t pt-6">
          <DataSection onChanged={onChanged} />
        </div>
      </DialogContent>
    </Dialog>
  );
}
