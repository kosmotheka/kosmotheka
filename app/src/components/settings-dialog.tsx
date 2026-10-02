import { useTranslation } from "react-i18next";
import { ChevronRight, HardDrive, LogIn } from "lucide-react";

import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Button } from "@/components/ui/button";
import { supportedLanguages } from "@/i18n";
import { refreshAccount, useAccount } from "@/lib/account";
import { signOut } from "@/lib/api";
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
  const account = useAccount();
  const host = account.official ? "kosmotheka.app" : account.server.replace(/^https?:\/\//, "");

  return (
    <ResponsiveDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t("settings.title")}
      description={t("settings.description")}
    >
      <div className="space-y-6">
        <div className="space-y-2">
          <p className="text-sm font-medium">{t("account.settings.cloudAndAccount")}</p>
          {account.signedIn && account.user ? (
            <div className="flex items-center gap-3 rounded-lg border p-3">
              <span className="flex size-10 shrink-0 items-center justify-center rounded-full bg-secondary text-sm font-semibold">
                {account.user.name.slice(0, 1).toUpperCase()}
              </span>
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="text-sm font-medium">{account.user.name}</span>
                <span className="text-sm text-muted-foreground">
                  {t("account.settings.connected", { host })}
                </span>
              </span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => void signOut().then(() => refreshAccount())}
              >
                {t("account.settings.signOut")}
              </Button>
            </div>
          ) : (
            <div className="flex flex-col gap-3 rounded-lg border p-3">
              <div className="flex gap-3">
                <span className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-secondary">
                  <HardDrive className="size-[19px]" />
                </span>
                <span className="flex flex-col gap-0.5">
                  <span className="text-sm font-medium">{t("account.settings.noneTitle")}</span>
                  <span className="text-sm text-muted-foreground">
                    {t("account.settings.noneBody")}
                  </span>
                </span>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => openOverlay({ name: "auth" })}
                >
                  <LogIn /> {t("account.settings.signInOrCreate")}
                </Button>
                <span className="text-sm text-muted-foreground">
                  {t("account.settings.optionalHint")}
                </span>
              </div>
            </div>
          )}
        </div>
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
