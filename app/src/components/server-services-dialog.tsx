import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Server } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { useAccount } from "@/lib/account";
import {
  type ServerServices,
  type Settings,
  type Source,
  getSettings,
  markServerServicesSeen,
  saveSettings,
  serverServices,
} from "@/lib/api";

/** The sources behind each service a server offers (`/v1/server`): BoardGameGeek also answers for RPGGeek. */
const SOURCES_OF: Record<string, Source[]> = {
  google_books: ["google_books"],
  tmdb: ["tmdb"],
  rawg: ["rawg"],
  comic_vine: ["comic_vine"],
  bgg: ["board_game_geek", "rpg_geek"],
  gameupc: ["game_upc"],
};

function hostOf(server: string): string {
  try {
    return new URL(server).host;
  } catch {
    return server;
  }
}

/**
 * The welcome's screen for the search services that go through the server (D91): it shows once a session is open
 * on a server that offers them, and again when that server offers a new one or the device moves to another server.
 * Each service has its switch, the same ones as in Ajustes › Proveedores de datos. `blocked`: something else is on
 * screen (the welcome, the news), so it waits.
 */
export function ServerServicesDialog({ blocked }: { blocked: boolean }) {
  const { t } = useTranslation();
  const account = useAccount();
  // Tied to the server it was asked of: an answer from another session or server never shows.
  const [asked, setAsked] = useState<{ server: string; services: ServerServices } | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);

  useEffect(() => {
    if (!account.signedIn) return;
    const server = account.server;
    let current = true;
    void serverServices()
      .then((answer) => {
        if (!current || answer.new.length === 0) return;
        setAsked({ server, services: answer });
        void getSettings().then((loaded) => current && setSettings(loaded));
      })
      // Without an answer nothing is offered through the server either: nothing to tell.
      .catch(() => {});
    return () => {
      current = false;
    };
  }, [account.signedIn, account.server]);

  const services = account.signedIn && asked?.server === account.server ? asked.services : null;
  if (!services || blocked) return null;

  // Told before on this server, so the new ones are marked.
  const someSeen = services.new.length < services.offered.length;
  const rows = services.offered.flatMap((service) =>
    (SOURCES_OF[service] ?? []).map((source) => ({ service, source })),
  );

  const isOn = (source: Source): boolean => {
    if (!settings) return true;
    if (source === "google_books") return settings.googleBooksViaServer;
    return !settings.disabledSources.includes(source);
  };

  const toggle = (source: Source, on: boolean) => {
    if (!settings) return;
    let next: Settings;
    if (source === "google_books") {
      next = { ...settings, googleBooksViaServer: on };
    } else {
      const disabled = new Set(settings.disabledSources);
      if (on) disabled.delete(source);
      else disabled.add(source);
      next = { ...settings, disabledSources: [...disabled] };
    }
    setSettings(next);
    void saveSettings(next);
  };

  const done = () => {
    void markServerServicesSeen(services.offered);
    setAsked(null);
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="server-services-title"
      className="fixed inset-0 z-[150] flex flex-col bg-background pt-[env(safe-area-inset-top)] pb-[env(safe-area-inset-bottom)]"
    >
      <div className="flex flex-1 flex-col items-center justify-center overflow-y-auto px-6 py-8">
        <div className="flex w-full max-w-md flex-col gap-4">
          <span className="flex size-14 items-center justify-center rounded-2xl bg-secondary">
            <Server className="size-7" />
          </span>
          <h1 id="server-services-title" className="text-2xl font-bold tracking-tight">
            {t("serverServices.title")}
          </h1>
          <p className="text-[15px] leading-relaxed text-muted-foreground">
            {t("serverServices.body", { server: hostOf(account.server) })}
          </p>
          <ul className="flex flex-col divide-y rounded-[14px] border">
            {rows.map(({ service, source }) => (
              <li key={source} className="flex items-center gap-3 px-3.5 py-3">
                <label htmlFor={`server-${source}`} className="flex flex-1 flex-col gap-0.5">
                  <span className="flex items-center gap-2 text-sm font-medium">
                    {t(`lookup.sources.${source}`)}
                    {someSeen && services.new.includes(service) && (
                      <Badge variant="secondary">{t("serverServices.new")}</Badge>
                    )}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {t(`serverServices.sources.${source}`)}
                  </span>
                </label>
                <Switch
                  id={`server-${source}`}
                  disabled={!settings}
                  checked={isOn(source)}
                  onCheckedChange={(on) => toggle(source, on)}
                />
              </li>
            ))}
          </ul>
          <p className="text-xs text-muted-foreground">{t("serverServices.later")}</p>
        </div>
      </div>
      <div className="flex w-full justify-end px-6 pb-6">
        <Button onClick={done}>{t("serverServices.done")}</Button>
      </div>
    </div>
  );
}
