import { type FormEvent, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Field } from "@/components/book-form";
import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { supportedLanguages } from "@/i18n";
import { getSettings, saveSettings } from "@/lib/api";

/** Each language in its own name, so it can be found whatever the current one. */
const LANGUAGE_NAMES: Record<(typeof supportedLanguages)[number], string> = {
  en: "English",
  es: "Español",
};

export function SettingsDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t, i18n } = useTranslation();
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
    <ResponsiveDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t("settings.title")}
      description={t("settings.description")}
    >
      <form onSubmit={submit} className="space-y-6">
        <div className="space-y-2">
          <p className="text-sm font-medium">{t("nav.languages")}</p>
          <div className="flex gap-2">
            {supportedLanguages.map((language) => (
              <Button
                key={language}
                type="button"
                size="sm"
                variant={i18n.resolvedLanguage === language ? "secondary" : "outline"}
                onClick={() => void i18n.changeLanguage(language)}
              >
                {LANGUAGE_NAMES[language]}
              </Button>
            ))}
          </div>
        </div>
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
        <div className="flex justify-end gap-2">
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            {t("form.cancel")}
          </Button>
          <Button type="submit">{t("form.save")}</Button>
        </div>
      </form>
    </ResponsiveDialog>
  );
}
