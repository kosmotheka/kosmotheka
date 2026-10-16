import { useState } from "react";
import { useTranslation } from "react-i18next";
import { LibraryBig, ScanBarcode, Search, UserRound, Users } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import {
  type Settings,
  type Source,
  beginWithName,
  getSettings,
  isMobile,
  personName,
  saveSettings,
} from "@/lib/api";
import { cn } from "@/lib/utils";

/** Raise it when the welcome adds a search service, so everyone sees it again (D91). */
export const WELCOME_VERSION = 2;

/** The services the app asks without a key, so they are on unless turned off. */
const KEYLESS_SOURCES: Source[] = [
  "bne",
  "library_of_congress",
  "open_library",
  "inventaire",
  "wikidata",
  "music_brainz",
];

type Step = "name" | "local" | "scan" | "search" | "share";

/**
 * First-run welcome (D91): the person's name first (D97), then what stays on the device, scanning, and which
 * public catalogs the app asks, each one with its switch, so nothing happens unannounced. Until the name is
 * given there is no library, so it cannot be skipped. `onlyName`: the rest was seen before.
 */
export function WelcomeDialog({
  open,
  onlyName = false,
  onDone,
}: {
  open: boolean;
  onlyName?: boolean;
  onDone: () => void;
}) {
  const { t } = useTranslation();
  const [mobile, setMobile] = useState(false);
  const [index, setIndex] = useState(0);
  const [settings, setSettings] = useState<Settings | null>(null);
  // `null`: still asking; empty or a name: what the person typed or already had.
  const [named, setNamed] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [seeded, setSeeded] = useState(false);

  // Fresh each time it opens, like ProvidersDialog.
  if (open && !seeded) {
    setSeeded(true);
    setIndex(0);
    void isMobile().then(setMobile);
    void getSettings().then(setSettings);
    setError(null);
    void personName().then((name) => {
      setNamed(name ?? "");
      setDraft(name ?? "");
    });
  }
  if (!open && seeded) setSeeded(false);

  if (!open) return null;

  const rest: Step[] = onlyName
    ? []
    : mobile
      ? ["local", "scan", "search", "share"]
      : ["local", "search", "share"];
  // Asked while there is no name yet; a name that is already there is not asked again.
  const steps: Step[] = named === "" ? ["name", ...rest] : rest;
  if (steps.length === 0) return null;
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

  const Icon = {
    name: UserRound,
    local: LibraryBig,
    scan: ScanBarcode,
    search: Search,
    share: Users,
  }[step];

  const next = async () => {
    if (step === "name") {
      setBusy(true);
      setError(null);
      try {
        await beginWithName(draft.trim());
        setNamed(draft.trim());
        // The name step is gone from the list now: the next one is at the same place.
        if (rest.length === 0) onDone();
        else setIndex(0);
      } catch (e) {
        setError(t("welcome.name.invalid"));
        console.error(e);
      } finally {
        setBusy(false);
      }
      return;
    }
    if (last) onDone();
    else setIndex(index + 1);
  };

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
          {step === "name" && (
            <label className="flex flex-col gap-1.5 text-sm font-medium">
              {t("welcome.name.label")}
              <Input
                autoFocus
                value={draft}
                maxLength={100}
                onChange={(e) => setDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && draft.trim() && !busy) void next();
                }}
              />
            </label>
          )}
          {step === "name" && (
            <p className="text-xs text-muted-foreground">{t("welcome.name.hint")}</p>
          )}
          {error && <p className="text-sm text-destructive">{error}</p>}
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
          {!last && step !== "name" && (
            <Button variant="ghost" onClick={onDone}>
              {t("welcome.skip")}
            </Button>
          )}
          <Button disabled={busy || (step === "name" && !draft.trim())} onClick={() => void next()}>
            {t(last ? "welcome.start" : "welcome.next")}
          </Button>
        </div>
      </div>
    </div>
  );
}
