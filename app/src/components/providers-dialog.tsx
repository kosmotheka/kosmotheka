import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronLeft } from "lucide-react";

import { Switch } from "@/components/people/role-parts";
import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { useDesktop } from "@/components/shell/use-desktop";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  type Kind,
  type Settings,
  type Source,
  kinds,
  cloudMetadataOffers,
  getSettings,
  lookupCode,
  lookupIsbn,
  saveSettings,
} from "@/lib/api";
import { cn } from "@/lib/utils";
import tmdbLogo from "@/assets/tmdb-logo.svg";

const SOURCES_BY_KIND: Record<Kind, Source[]> = {
  book: ["bne", "open_library", "inventaire", "wikidata", "google_books"],
  comic: ["wikidata"],
  boardgame: ["wikidata", "game_upc"],
  film: ["tmdb", "wikidata"],
  tv_series: ["tmdb", "wikidata"],
  videogame: ["wikidata"],
  music: ["music_brainz"],
  other: [],
};

const KEYED_SOURCES = new Set<Source>(["google_books", "game_upc"]);

/** Sources that only arrive with an account: the server of the account asks them (D62). */
const ACCOUNT_SOURCES = new Set<Source>(["tmdb"]);

const GET_KEY_URL: Partial<Record<Source, string>> = {
  google_books: "https://developers.google.com/books/docs/v1/using#APIKey",
  game_upc: "https://gameupc.com",
};

/** Known codes that resolve to a real result on that provider, used only to
 * check a stored key without depending on the person's own data. */
async function testSource(source: Source): Promise<"ok" | "bad_key" | "failed"> {
  try {
    const result =
      source === "google_books"
        ? await lookupIsbn("9780140328721")
        : await lookupCode("019962194719");
    return result.warnings.some((w) => w.source === source && w.kind === "bad_key")
      ? "bad_key"
      : "ok";
  } catch {
    return "failed";
  }
}

/** What the server of the session looks up for its accounts (7.8); empty without a session. */
interface Offers {
  googleBooks: boolean;
  tmdb: boolean;
}

function usableCount(kind: Kind, settings: Settings, offers: Offers): number {
  return SOURCES_BY_KIND[kind].filter((source) => isUsable(source, settings, offers)).length;
}

/** Google Books works without a key of their own when the account's server looks it up (7.8). */
function needsOwnKey(source: Source, settings: Settings, offers: Offers): boolean {
  if (!KEYED_SOURCES.has(source)) return false;
  return !(source === "google_books" && offers.googleBooks && settings.googleBooksViaServer);
}

function isUsable(source: Source, settings: Settings, offers: Offers): boolean {
  if (settings.disabledSources.includes(source)) return false;
  if (ACCOUNT_SOURCES.has(source)) return offers.tmdb;
  return !needsOwnKey(source, settings, offers) || Boolean(settings.apiKeys[source]?.trim());
}

export function ProvidersDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation();
  const desktop = useDesktop();
  const [settings, setSettings] = useState<Settings | null>(null);
  const [selectedKind, setSelectedKind] = useState<Kind | null>(null);
  const [keyDrafts, setKeyDrafts] = useState<Record<string, string>>({});
  const [testStatus, setTestStatus] = useState<
    Partial<Record<Source, "testing" | "ok" | "bad_key" | "failed">>
  >({});
  const [seeded, setSeeded] = useState(false);
  // What the server of the session looks up for its accounts (7.8): only then are those shown as available.
  const [offers, setOffers] = useState<Offers>({ googleBooks: false, tmdb: false });

  // Fresh settings each time the page opens, as Design asks (same pattern as PairDialog).
  if (open && !seeded) {
    setSeeded(true);
    setSelectedKind(desktop ? "book" : null);
    void getSettings().then((loaded) => {
      setSettings(loaded);
      setKeyDrafts(loaded.apiKeys);
    });
    void cloudMetadataOffers()
      .then((offered) =>
        setOffers({
          googleBooks: offered.includes("google_books"),
          tmdb: offered.includes("tmdb"),
        }),
      )
      .catch(() => setOffers({ googleBooks: false, tmdb: false }));
  }
  if (!open && seeded) {
    setSeeded(false);
    setSettings(null);
    setOffers({ googleBooks: false, tmdb: false });
  }

  const update = async (next: Settings) => {
    setSettings(next);
    await saveSettings(next);
  };

  const toggleSource = (source: Source) => {
    if (!settings) return;
    const disabled = new Set(settings.disabledSources);
    if (disabled.has(source)) disabled.delete(source);
    else disabled.add(source);
    void update({ ...settings, disabledSources: [...disabled] });
  };

  const toggleViaCloud = () => {
    if (!settings) return;
    void update({ ...settings, googleBooksViaServer: !settings.googleBooksViaServer });
  };

  const saveKey = (source: Source) => {
    if (!settings) return;
    const value = (keyDrafts[source] ?? "").trim();
    void update({ ...settings, apiKeys: { ...settings.apiKeys, [source]: value } });
    setTestStatus((s) => ({ ...s, [source]: undefined }));
  };

  const runTest = async (source: Source) => {
    setTestStatus((s) => ({ ...s, [source]: "testing" }));
    const status = await testSource(source);
    setTestStatus((s) => ({ ...s, [source]: status }));
  };

  const kindLabel = (kind: Kind) => t(`typesPlural.${kind}`);

  const kindsList = (
    <nav aria-label={t("providers.title")} className="flex flex-col gap-0.5">
      {kinds.map((kind) => {
        const sources = SOURCES_BY_KIND[kind];
        const n = settings ? usableCount(kind, settings, offers) : 0;
        const weak = sources.length > 0 && n < 2;
        return (
          <button
            key={kind}
            type="button"
            onClick={() => setSelectedKind(kind)}
            className={cn(
              "flex h-11 items-center gap-2.5 rounded-lg px-3 text-left text-sm",
              desktop && selectedKind === kind ? "bg-secondary font-semibold" : "font-medium",
            )}
          >
            <span className="flex-1">{kindLabel(kind)}</span>
            {sources.length > 0 ? (
              <span
                className={cn(
                  "text-xs",
                  weak ? "font-medium text-amber-600 dark:text-amber-400" : "text-muted-foreground",
                )}
              >
                {t("providers.fewSources", { count: n, total: sources.length })}
              </span>
            ) : (
              <span className="text-xs text-muted-foreground">{t("providers.noSources")}</span>
            )}
            {!desktop && (
              <ChevronLeft className="size-4 shrink-0 rotate-180 text-muted-foreground" />
            )}
          </button>
        );
      })}
    </nav>
  );

  const sourceCards = (kind: Kind) => {
    const sources = SOURCES_BY_KIND[kind];
    if (sources.length === 0) {
      return <p className="text-sm text-muted-foreground">{t("providers.other")}</p>;
    }
    return (
      <div className="flex flex-col gap-3">
        {sources.map((source) => {
          const keyed = KEYED_SOURCES.has(source);
          const hasKey = Boolean(settings?.apiKeys[source]?.trim());
          const accountOnly = ACCOUNT_SOURCES.has(source);
          const on = settings ? isUsable(source, settings, offers) : false;
          const canToggle = accountOnly
            ? offers.tmdb
            : !settings || !needsOwnKey(source, settings, offers) || hasKey;
          const cloudSwitch = source === "google_books" && offers.googleBooks && settings !== null;
          const status = testStatus[source];
          return (
            <div key={source} className="flex flex-col gap-2.5 rounded-lg border p-3.5">
              <div className="flex items-center gap-3">
                <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                  <span className="flex items-center gap-2 text-sm font-semibold">
                    {t(`lookup.sources.${source}`)}
                    <span className="rounded-full border px-2 py-px text-xs font-medium text-muted-foreground">
                      {t(
                        accountOnly
                          ? "providers.needsAccountBadge"
                          : keyed
                            ? "providers.needsKeyBadge"
                            : "providers.noKeyBadge",
                      )}
                    </span>
                  </span>
                  <span className="text-[13px] text-muted-foreground">
                    {t(`providers.descriptions.${kind}.${source}`)}
                  </span>
                </div>
                <Switch
                  checked={on}
                  disabled={!canToggle}
                  label={t("providers.toggleAria", { name: t(`lookup.sources.${source}`) })}
                  onChange={() => toggleSource(source)}
                />
              </div>
              {accountOnly && (
                <p className="text-[13px] text-muted-foreground">
                  {t(offers.tmdb ? "providers.tmdbSends" : "providers.tmdbWithAccount")}
                </p>
              )}
              {cloudSwitch && (
                <div className="flex items-center gap-3 border-t pt-2.5">
                  <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                    <span className="text-sm font-medium">{t("providers.viaCloud")}</span>
                    <span className="text-[13px] text-muted-foreground">
                      {t("providers.viaCloudHelp")}
                    </span>
                  </div>
                  <Switch
                    checked={settings.googleBooksViaServer}
                    label={t("providers.viaCloud")}
                    onChange={toggleViaCloud}
                  />
                </div>
              )}
              {keyed && (
                <div className="flex flex-wrap items-center gap-2">
                  <Input
                    type="password"
                    autoComplete="off"
                    className="max-w-64"
                    placeholder={t("providers.keyPlaceholder")}
                    value={keyDrafts[source] ?? ""}
                    onChange={(e) => setKeyDrafts((d) => ({ ...d, [source]: e.target.value }))}
                  />
                  <Button type="button" size="sm" variant="outline" onClick={() => saveKey(source)}>
                    {t("providers.save")}
                  </Button>
                  {hasKey && (
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      disabled={status === "testing"}
                      onClick={() => void runTest(source)}
                    >
                      {t("providers.test")}
                    </Button>
                  )}
                  {GET_KEY_URL[source] && (
                    <a
                      href={GET_KEY_URL[source]}
                      target="_blank"
                      rel="noreferrer"
                      className="text-[13px] text-primary hover:underline"
                    >
                      {t("providers.getKey")}
                    </a>
                  )}
                  {status && status !== "testing" && (
                    <p
                      className={cn(
                        "w-full text-[13px]",
                        status === "ok" ? "text-muted-foreground" : "text-destructive",
                      )}
                    >
                      {t(
                        `providers.test${status === "ok" ? "Ok" : status === "bad_key" ? "BadKey" : "Failed"}`,
                      )}
                    </p>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    );
  };

  const body = !settings ? null : desktop ? (
    <div className="flex gap-6">
      <div className="w-56 shrink-0">{kindsList}</div>
      <div className="min-w-0 flex-1 space-y-3">
        <h2 className="text-base font-semibold">{selectedKind ? kindLabel(selectedKind) : ""}</h2>
        {selectedKind && sourceCards(selectedKind)}
      </div>
    </div>
  ) : selectedKind ? (
    <div className="space-y-3">
      <button
        type="button"
        className="flex items-center gap-1 text-sm font-medium text-primary"
        onClick={() => setSelectedKind(null)}
      >
        <ChevronLeft className="size-4" />
        {t("providers.title")}
      </button>
      {sourceCards(selectedKind)}
    </div>
  ) : (
    kindsList
  );

  const attribution = (
    <section className="flex flex-col gap-2 border-t pt-3.5">
      <h3 className="text-sm font-semibold">{t("providers.dataSources")}</h3>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
        <img src={tmdbLogo} alt="TMDB" className="h-3.5 w-auto" />
        <p className="min-w-0 flex-1 text-xs text-muted-foreground">{t("providers.tmdbNotice")}</p>
      </div>
    </section>
  );

  return (
    <ResponsiveDialog
      open={open}
      onOpenChange={onOpenChange}
      title={!desktop && selectedKind ? kindLabel(selectedKind) : t("providers.title")}
      description={t("providers.description")}
      className="sm:max-w-3xl"
    >
      {/* The dialog scrolls as a whole: nothing here may shrink, or what follows is drawn over the list. */}
      <div className="flex flex-col gap-4">
        <div className="flex items-center gap-2.5 rounded-lg border p-3 text-sm">
          <span className="flex-1">
            {t(offers.googleBooks || offers.tmdb ? "providers.withCloud" : "providers.noCloud")}
          </span>
        </div>
        {body}
        {attribution}
      </div>
    </ResponsiveDialog>
  );
}
