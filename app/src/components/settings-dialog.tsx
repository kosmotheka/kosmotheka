import { useTranslation } from "react-i18next";
import { ChevronRight } from "lucide-react";

import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Button } from "@/components/ui/button";
import { supportedLanguages } from "@/i18n";
import { openOverlay } from "@/lib/navigation";

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

  return (
    <ResponsiveDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t("settings.title")}
      description={t("settings.description")}
    >
      <div className="space-y-6">
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
        <button
          type="button"
          className="flex w-full items-center gap-3 rounded-lg border p-3 text-left hover:bg-accent"
          onClick={() => openOverlay({ name: "providers" })}
        >
          <span className="flex-1">
            <span className="block text-sm font-medium">{t("settings.providers")}</span>
            <span className="block text-sm text-muted-foreground">
              {t("settings.providersHelp")}
            </span>
          </span>
          <ChevronRight className="size-4 shrink-0 text-muted-foreground" />
        </button>
        <div className="flex justify-end">
          <Button type="button" onClick={() => onOpenChange(false)}>
            {t("common.close")}
          </Button>
        </div>
      </div>
    </ResponsiveDialog>
  );
}
