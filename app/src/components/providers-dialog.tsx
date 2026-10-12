import { useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { ChevronLeft, Trash2 } from "lucide-react";

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
  searchWorks,
} from "@/lib/api";
import { cn } from "@/lib/utils";
import { ExternalLink } from "@/components/external-link";
import { SOURCES_WITH_A_LINK } from "@/components/source-credits";
import tmdbLogo from "@/assets/tmdb-logo.svg";

const SOURCES_BY_KIND: Record<Kind, Source[]> = {
  book: ["bne", "open_library", "inventaire", "wikidata", "google_books"],
  comic: ["comic_vine", "wikidata"],
  boardgame: ["wikidata", "game_upc"],
  film: ["tmdb", "wikidata"],
  tv_series: ["tmdb", "wikidata"],
  videogame: ["rawg", "wikidata"],
  music: ["music_brainz"],
  other: [],
};

/** Every source once, in the order the types list them, and the types each one covers. */
const ALL_SOURCES: Source[] = [...new Set(kinds.flatMap((kind) => SOURCES_BY_KIND[kind]))];

function kindsOf(source: Source): Kind[] {
  return kinds.filter((kind) => SOURCES_BY_KIND[kind].includes(source));
}

/** What is selected on the left: every source at once, one type, or (on a phone) nothing yet. */
type Selection = Kind | "all" | null;

const KEYED_SOURCES = new Set<Source>(["google_books", "game_upc", "tmdb", "rawg", "comic_vine"]);

/** Sources the account's server can also ask when there is no key of their own (D62, D83). */
const ACCOUNT_SOURCES = new Set<Source>(["tmdb", "rawg", "comic_vine"]);

const GET_KEY_URL: Partial<Record<Source, string>> = {
  google_books: "https://developers.google.com/books/docs/v1/using#APIKey",
  game_upc: "https://gameupc.com",
  tmdb: "https://www.themoviedb.org/settings/api",
  rawg: "https://rawg.io/apidocs",
  comic_vine: "https://comicvine.gamespot.com/api/",
};

/** Known codes that resolve to a real result on that provider, used only to
 * check a stored key without depending on the person's own data. */
async function testSource(source: Source): Promise<"ok" | "bad_key" | "failed"> {
  try {
    const result =
      source === "google_books"
        ? await lookupIsbn("9780140328721")
        : source === "tmdb"
          ? await searchWorks("film", "matrix", "en")
          : source === "rawg"
            ? await searchWorks("videogame", "zelda", "en")
            : source === "comic_vine"
              ? await searchWorks("comic", "batman", "en")
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
  rawg: boolean;
  comicVine: boolean;
}

const NO_OFFERS: Offers = { googleBooks: false, tmdb: false, rawg: false, comicVine: false };

function usableCount(sources: Source[], settings: Settings, offers: Offers): number {
  return sources.filter((source) => isUsable(source, settings, offers)).length;
}

/** Whether the account's server asks this source instead of a key of their own (7.8). */
function viaAccount(source: Source, settings: Settings, offers: Offers): boolean {
  if (source === "google_books") return offers.googleBooks && settings.googleBooksViaServer;
  if (source === "tmdb") return offers.tmdb;
  if (source === "rawg") return offers.rawg;
  if (source === "comic_vine") return offers.comicVine;
  return false;
}

/** A keyed source works with a key of their own or, where the account's server asks it, without one. */
function isAvailable(source: Source, settings: Settings, offers: Offers): boolean {
  if (!KEYED_SOURCES.has(source)) return true;
  return Boolean(settings.apiKeys[source]?.trim()) || viaAccount(source, settings, offers);
}

function isUsable(source: Source, settings: Settings, offers: Offers): boolean {
  return !settings.disabledSources.includes(source) && isAvailable(source, settings, offers);
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
  const [selectedKind, setSelectedKind] = useState<Selection>(null);
  const [keyDrafts, setKeyDrafts] = useState<Record<string, string>>({});
  const [testStatus, setTestStatus] = useState<
    Partial<Record<Source, "testing" | "ok" | "bad_key" | "failed">>
  >({});
  const [seeded, setSeeded] = useState(false);
  // What the server of the session looks up for its accounts (7.8): only then are those shown as available.
  const [offers, setOffers] = useState<Offers>(NO_OFFERS);

  // Fresh settings each time the page opens, as Design asks (same pattern as PairDialog).
  if (open && !seeded) {
    setSeeded(true);
    setSelectedKind(desktop ? "all" : null);
    void getSettings().then((loaded) => {
      setSettings(loaded);
      setKeyDrafts(loaded.apiKeys);
    });
    void cloudMetadataOffers()
      .then((offered) =>
        setOffers({
          googleBooks: offered.includes("google_books"),
          tmdb: offered.includes("tmdb"),
          rawg: offered.includes("rawg"),
          comicVine: offered.includes("comic_vine"),
        }),
      )
      .catch(() => setOffers(NO_OFFERS));
  }
  if (!open && seeded) {
    setSeeded(false);
    setSettings(null);
    setOffers(NO_OFFERS);
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

  const runTest = async (source: Source) => {
    setTestStatus((s) => ({ ...s, [source]: "testing" }));
    const status = await testSource(source);
    setTestStatus((s) => ({ ...s, [source]: status }));
  };

  // A key is tried the moment it is saved: otherwise anything typed here looks accepted until the next search fails.
  const saveKey = async (source: Source) => {
    if (!settings) return;
    const value = (keyDrafts[source] ?? "").trim();
    setTestStatus((s) => ({ ...s, [source]: undefined }));
    await update({ ...settings, apiKeys: { ...settings.apiKeys, [source]: value } });
    if (value) await runTest(source);
  };

  // Taking the key away is not switching the source off: it stays on, and works again if the account's server offers it.
  const removeKey = (source: Source) => {
    if (!settings) return;
    setKeyDrafts((d) => ({ ...d, [source]: "" }));
    void update({ ...settings, apiKeys: { ...settings.apiKeys, [source]: "" } });
    setTestStatus((s) => ({ ...s, [source]: undefined }));
  };

  const kindLabel = (kind: Kind) => t(`typesPlural.${kind}`);
  const selectionLabel = (selection: Kind | "all") =>
    selection === "all" ? t("providers.all") : kindLabel(selection);

  const kindsList = (
    <nav aria-label={t("providers.title")} className="flex flex-col gap-0.5">
      {(["all", ...kinds] as const).map((entry) => {
        const sources = entry === "all" ? ALL_SOURCES : SOURCES_BY_KIND[entry];
        const n = settings ? usableCount(sources, settings, offers) : 0;
        const weak = entry !== "all" && sources.length > 0 && n < 2;
        return (
          <button
            key={entry}
            type="button"
            onClick={() => setSelectedKind(entry)}
            className={cn(
              "flex h-11 items-center gap-2.5 rounded-lg px-3 text-left text-sm",
              desktop && selectedKind === entry ? "bg-secondary font-semibold" : "font-medium",
            )}
          >
            <span className="flex-1">{selectionLabel(entry)}</span>
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

  const sourceCards = (selection: Kind | "all") => {
    const sources = selection === "all" ? ALL_SOURCES : SOURCES_BY_KIND[selection];
    if (sources.length === 0) {
      return <p className="text-sm text-muted-foreground">{t("providers.other")}</p>;
    }
    return (
      <div className="flex flex-col gap-3">
        {sources.map((source) => {
          const keyed = KEYED_SOURCES.has(source);
          const hasKey = Boolean(settings?.apiKeys[source]?.trim());
          const accountToo = ACCOUNT_SOURCES.has(source);
          // The switch is what the person chose, never whether the source can work now: taking a key away must not
          // look like switching the source off. A source that is on and cannot work says so below.
          const on = settings ? !settings.disabledSources.includes(source) : false;
          const stuck = settings !== null && on && !isAvailable(source, settings, offers);
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
                        accountToo
                          ? "providers.needsKeyOrAccountBadge"
                          : keyed
                            ? "providers.needsKeyBadge"
                            : "providers.noKeyBadge",
                      )}
                    </span>
                  </span>
                  <span className="text-[13px] text-muted-foreground">
                    {selection === "all"
                      ? t(`providers.sourceDescriptions.${source}`)
                      : t(`providers.descriptions.${selection}.${source}`)}
                  </span>
                  {selection === "all" && (
                    <span className="text-xs text-muted-foreground">
                      {kindsOf(source).map(kindLabel).join(" · ")}
                    </span>
                  )}
                </div>
                <Switch
                  checked={on}
                  disabled={!settings}
                  label={t("providers.toggleAria", { name: t(`lookup.sources.${source}`) })}
                  onChange={() => toggleSource(source)}
                />
              </div>
              {stuck && !accountToo && (
                <p className="text-[13px] font-medium text-amber-600 dark:text-amber-400">
                  {t("providers.missingKey")}
                </p>
              )}
              {accountToo && settings && (
                <p
                  className={cn(
                    "text-[13px]",
                    stuck
                      ? "font-medium text-amber-600 dark:text-amber-400"
                      : "text-muted-foreground",
                  )}
                >
                  {t(
                    hasKey
                      ? "providers.ownKeyUsed"
                      : viaAccount(source, settings, offers)
                        ? `providers.${source}Sends`
                        : `providers.${source}WithAccount`,
                  )}
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
                    placeholder={t(
                      source === "tmdb"
                        ? "providers.keyPlaceholderTmdb"
                        : "providers.keyPlaceholder",
                    )}
                    value={keyDrafts[source] ?? ""}
                    onChange={(e) => setKeyDrafts((d) => ({ ...d, [source]: e.target.value }))}
                  />
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={() => void saveKey(source)}
                  >
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
                  {hasKey && (
                    <Button
                      type="button"
                      size="icon-sm"
                      variant="ghost"
                      aria-label={t("providers.removeKey", { name: t(`lookup.sources.${source}`) })}
                      title={t("providers.removeKey", { name: t(`lookup.sources.${source}`) })}
                      onClick={() => removeKey(source)}
                    >
                      <Trash2 />
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
        <h2 className="text-base font-semibold">
          {selectedKind ? selectionLabel(selectedKind) : ""}
        </h2>
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

  // What each provider asks us to show: a footer, small and quiet, whatever is selected above.
  const credits = (
    <section className="flex flex-col gap-1.5 border-t pt-3 text-xs text-muted-foreground">
      <h3 className="font-medium">{t("providers.credits")}</h3>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5">
        <img src={tmdbLogo} alt="TMDB" className="h-3 w-auto" />
        <p className="min-w-0 flex-1">{t("providers.tmdbNotice")}</p>
      </div>
      {Object.values(SOURCES_WITH_A_LINK).map((credit) => (
        <p key={credit.url}>
          <Trans i18nKey={credit.notice} components={{ link: <ExternalLink url={credit.url} /> }} />
        </p>
      ))}
    </section>
  );

  return (
    <ResponsiveDialog
      open={open}
      onOpenChange={onOpenChange}
      title={!desktop && selectedKind ? selectionLabel(selectedKind) : t("providers.title")}
      description={t("providers.description")}
      className="sm:max-w-3xl"
    >
      {/* The dialog scrolls as a whole: nothing here may shrink, or what follows is drawn over the list. */}
      <div className="flex flex-col gap-4">
        <div className="flex items-center gap-2.5 rounded-lg border p-3 text-sm">
          <span className="flex-1">
            {t(
              offers.googleBooks || offers.tmdb || offers.rawg || offers.comicVine
                ? "providers.withCloud"
                : "providers.noCloud",
            )}
          </span>
        </div>
        {body}
        {credits}
      </div>
    </ResponsiveDialog>
  );
}
