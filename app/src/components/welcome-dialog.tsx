import { useState } from "react";
import { useTranslation } from "react-i18next";
import { LibraryBig, ScanBarcode, Search, Users } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { type Settings, type Source, getSettings, isMobile, saveSettings } from "@/lib/api";
import { cn } from "@/lib/utils";

/** Raise it when the welcome adds a search service, so everyone sees it again (D91). */
export const WELCOME_VERSION = 1;

/** The services the app asks without a key, so they are on unless turned off. */
const KEYLESS_SOURCES: Source[] = [
  "bne",
  "library_of_congress",
  "open_library",
  "inventaire",
  "wikidata",
  "music_brainz",
];

type Step = "local" | "scan" | "search" | "share";

/**
 * First-run welcome (D91): what stays on the device, scanning, and which public
 * catalogs the app asks, each one with its switch, so nothing happens unannounced.
 */
export function WelcomeDialog({ open, onDone }: { open: boolean; onDone: () => void }) {
  const { t } = useTranslation();
  const [mobile, setMobile] = useState(false);
  const [index, setIndex] = useState(0);
  const [settings, setSettings] = useState<Settings | null>(null);

  const [seeded, setSeeded] = useState(false);

  // Fresh each time it opens, like ProvidersDialog.
  if (open && !seeded) {
    setSeeded(true);
    setIndex(0);
    void isMobile().then(setMobile);
    void getSettings().then(setSettings);
  }
  if (!open && seeded) setSeeded(false);

  if (!open) return null;

  const steps: Step[] = mobile
    ? ["local", "scan", "search", "share"]
    : ["local", "search", "share"];
  const step = steps[Math.min(index, steps.length - 1)] ?? "local";
  const last = index >= steps.length - 1;

  const toggle = (source: Source, on: boolean) => {
    if (!settings) return;
    const disabled = new Set(settings.disabledSources);
    if (on) disabled.delete(source);
    else disabled.add(source);
    const next = { ...settings, disabledSources: [...disabled] };
    setSettings(next);
    void saveSettings(next);
  };

  const Icon = { local: LibraryBig, scan: ScanBarcode, search: Search, share: Users }[step];

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="welcome-title"
      className="fixed inset-0 z-[150] flex flex-col bg-background pt-[env(safe-area-inset-top)] pb-[env(safe-area-inset-bottom)]"
    >
      <div className="flex flex-1 flex-col items-center justify-center overflow-y-auto px-6 py-8">
        <div className="flex w-full max-w-md flex-col gap-4">
          <span className="flex size-14 items-center justify-center rounded-2xl bg-secondary">
            <Icon className="size-7" />
          </span>
          <h1 id="welcome-title" className="text-2xl font-bold tracking-tight">
            {t(`welcome.${step}.title`)}
          </h1>
          <p className="text-[15px] leading-relaxed text-muted-foreground">
            {t(`welcome.${step}.body`)}
          </p>
          {step === "search" && (
            <ul className="flex flex-col divide-y rounded-[14px] border">
              {KEYLESS_SOURCES.map((source) => (
                <li key={source} className="flex items-center gap-3 px-3.5 py-3">
                  <label htmlFor={`welcome-${source}`} className="flex flex-1 flex-col gap-0.5">
                    <span className="text-sm font-medium">{t(`lookup.sources.${source}`)}</span>
                    <span className="text-xs text-muted-foreground">
                      {t(`welcome.sources.${source}`)}
                    </span>
                  </label>
                  <Switch
                    id={`welcome-${source}`}
                    disabled={!settings}
                    checked={settings ? !settings.disabledSources.includes(source) : true}
                    onCheckedChange={(on) => toggle(source, on)}
                  />
                </li>
              ))}
            </ul>
          )}
          {step === "search" && (
            <p className="text-xs text-muted-foreground">{t("welcome.search.later")}</p>
          )}
        </div>
      </div>
      <div className="flex w-full items-center justify-between gap-3 px-6 pb-6">
        <div className="flex gap-1.5" aria-hidden>
          {steps.map((s, i) => (
            <span
              key={s}
              className={cn("size-2 rounded-full", i === index ? "bg-foreground" : "bg-border")}
            />
          ))}
        </div>
        <div className="flex gap-2">
          {!last && (
            <Button variant="ghost" onClick={onDone}>
              {t("welcome.skip")}
            </Button>
          )}
          <Button onClick={() => (last ? onDone() : setIndex(index + 1))}>
            {t(last ? "welcome.start" : "welcome.next")}
          </Button>
        </div>
      </div>
    </div>
  );
}
