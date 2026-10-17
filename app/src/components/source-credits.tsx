import { Trans } from "react-i18next";

import { ExternalLink } from "@/components/external-link";
import type { Source } from "@/lib/api";
import bggLogo from "@/assets/powered-by-bgg.svg";
import bggLogoReversed from "@/assets/powered-by-bgg-reversed.svg";

/**
 * The sources that ask for a link back to them on every page where their data is used (D62): the sentence for the
 * results (`lookup`), the line for the credits of Providers, and where the link goes.
 */
export const SOURCES_WITH_A_LINK = {
  rawg: { text: "lookup.rawgData", notice: "providers.rawgNotice", url: "https://rawg.io" },
  comic_vine: {
    text: "lookup.comicVineData",
    notice: "providers.comicVineNotice",
    url: "https://comicvine.gamespot.com",
  },
} as const;

type Linked = keyof typeof SOURCES_WITH_A_LINK;

export const BGG_URL = "https://boardgamegeek.com";

/** Both sites of BoardGameGeek are credited together, with the same sentence and logo. */
const BGG_SOURCES: Source[] = ["board_game_geek", "rpg_geek"];

/** The «Powered by BGG» logo as BoardGameGeek supplies it, linked to BoardGameGeek: one for each theme, unmodified. */
export function BggLogo() {
  return (
    <ExternalLink url={BGG_URL}>
      <img src={bggLogo} alt="Powered by BGG" className="h-6 w-auto dark:hidden" />
      <img src={bggLogoReversed} alt="Powered by BGG" className="hidden h-6 w-auto dark:block" />
    </ExternalLink>
  );
}

function isLinked(source: Source): source is Linked {
  return source in SOURCES_WITH_A_LINK;
}

/** One quiet line for each of these sources that has a result on the page; nothing when none does. */
export function ResultsCredits({ sources }: { sources: Source[] }) {
  const present = [...new Set(sources)].filter(isLinked);
  const bgg = sources.some((source) => BGG_SOURCES.includes(source));
  if (present.length === 0 && !bgg) return null;
  return (
    <div className="px-2 pt-1 text-xs text-muted-foreground">
      {bgg && (
        <div className="flex flex-wrap items-center gap-2">
          <BggLogo />
          <p>
            <Trans i18nKey="lookup.bggData" components={{ link: <ExternalLink url={BGG_URL} /> }} />
          </p>
        </div>
      )}
      {present.map((source) => (
        <p key={source}>
          <Trans
            i18nKey={SOURCES_WITH_A_LINK[source].text}
            components={{ link: <ExternalLink url={SOURCES_WITH_A_LINK[source].url} /> }}
          />
        </p>
      ))}
    </div>
  );
}
