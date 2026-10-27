import { useState } from "react";
import { useTranslation } from "react-i18next";

import { ExternalLink } from "@/components/external-link";
import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Button } from "@/components/ui/button";
import { LicensesDialog } from "@/components/licenses-dialog";
import { ProvidersDialog } from "@/components/providers-dialog";
import { Switch } from "@/components/ui/switch";
import { WhatsNewDialog } from "@/components/whats-new-dialog";
import { type Language, languageOf } from "@/i18n";
import {
  type Settings,
  type WhatsNew,
  appVersion,
  checkForUpdate,
  checksForUpdates,
  getSettings,
  saveSettings,
  whatsNewRecent,
} from "@/lib/api";
import { downloadsPage } from "@/lib/updates";

const WEBSITE = "https://kosmotheka.com/";
/** The privacy policy in each language that has one; the others get the fallback language's. */
const PRIVACY: Partial<Record<Language, string>> & { en: string } = {
  es: "https://kosmotheka.com/es/privacidad.html",
  en: "https://kosmotheka.com/en/privacy.html",
};

/** What this app is and which version is installed: what somebody is asked for when something fails. */
export function AboutDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t, i18n } = useTranslation();
  const [version, setVersion] = useState<string | null>(null);
  const [seeded, setSeeded] = useState(false);
  const [news, setNews] = useState<WhatsNew | null>(null);
  const [licenses, setLicenses] = useState(false);
  // Opened from here, not as an overlay of its own (an overlay would take the place of this one): closing it comes back.
  const [providers, setProviders] = useState(false);
  // `null` where a store updates the app (a phone, a snap): there is nothing to show.
  const [settings, setSettings] = useState<Settings | null>(null);
  const [check, setCheck] = useState<
    { state: "idle" | "checking" | "upToDate" | "failed" } | { state: "available"; version: string }
  >({ state: "idle" });

  // Asked each time it opens, like ProvidersDialog.
  if (open && !seeded) {
    setSeeded(true);
    void appVersion()
      .then(setVersion)
      .catch(() => setVersion(null));
    setCheck({ state: "idle" });
    setSettings(null);
    void checksForUpdates()
      .then((here) => (here ? getSettings().then(setSettings) : undefined))
      .catch(() => setSettings(null));
  }
  if (!open && seeded) setSeeded(false);

  const privacy = PRIVACY[languageOf(i18n.language)] ?? PRIVACY.en;

  const switchUpdates = (on: boolean) => {
    if (!settings) return;
    const next = { ...settings, checkUpdates: on };
    setSettings(next);
    saveSettings(next).catch(() => setSettings(settings));
  };

  const checkNow = () => {
    setCheck({ state: "checking" });
    checkForUpdate(true)
      .then((found) =>
        setCheck(found ? { state: "available", version: found } : { state: "upToDate" }),
      )
      .catch(() => setCheck({ state: "failed" }));
  };

  return (
    <ResponsiveDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t("about.title")}
      className="sm:max-w-lg"
    >
      <div className="flex flex-col gap-4 text-sm">
        <div>
          <p className="text-lg font-semibold">Kosmotheka</p>
          <p className="text-muted-foreground tabular-nums">
            {version ? t("about.version", { version }) : t("about.versionUnknown")}
          </p>
        </div>
        <p>{t("about.summary")}</p>
        <p className="text-muted-foreground">{t("about.license")}</p>
        <ul className="flex flex-col gap-1.5">
          <li>
            <ExternalLink url={WEBSITE}>{t("about.website")}</ExternalLink>
          </li>
          <li>
            <ExternalLink url={privacy}>{t("about.privacy")}</ExternalLink>
          </li>
        </ul>
        {settings && (
          <section className="flex flex-col gap-3 rounded-lg border p-3">
            <div className="flex items-start gap-3">
              <label htmlFor="about-check-updates" className="flex flex-1 flex-col gap-0.5">
                <span className="font-medium">{t("updates.switch")}</span>
                <span className="text-xs text-muted-foreground">{t("updates.switchHint")}</span>
              </label>
              <Switch
                id="about-check-updates"
                checked={settings.checkUpdates}
                onCheckedChange={switchUpdates}
              />
            </div>
            <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
              <Button
                variant="outline"
                size="sm"
                disabled={check.state === "checking"}
                onClick={checkNow}
              >
                {t(check.state === "checking" ? "updates.checking" : "updates.checkNow")}
              </Button>
              <span role="status" className="text-muted-foreground">
                {check.state === "upToDate" && t("updates.upToDate")}
                {check.state === "failed" && t("updates.failed")}
                {check.state === "available" && (
                  <>
                    {t("updates.available", { version: check.version })}{" "}
                    <ExternalLink url={downloadsPage(i18n.language)}>
                      {t("updates.download")}
                    </ExternalLink>
                  </>
                )}
              </span>
            </div>
          </section>
        )}
        <div className="flex flex-wrap gap-2">
          <Button
            variant="outline"
            onClick={() =>
              void whatsNewRecent()
                .then(setNews)
                .catch(() => setNews(null))
            }
          >
            {t("about.whatsNew")}
          </Button>
          <Button variant="outline" onClick={() => setProviders(true)}>
            {t("about.sources")}
          </Button>
          <Button variant="outline" onClick={() => setLicenses(true)}>
            {t("about.licenses")}
          </Button>
        </div>
      </div>
      <LicensesDialog open={licenses} onOpenChange={setLicenses} />
      <ProvidersDialog open={providers} onOpenChange={setProviders} />
      {news && <WhatsNewDialog open news={news} onClose={() => setNews(null)} />}
    </ResponsiveDialog>
  );
}
