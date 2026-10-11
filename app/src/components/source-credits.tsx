import { Trans } from "react-i18next";

import { ExternalLink } from "@/components/external-link";
import type { Source } from "@/lib/api";

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

function isLinked(source: Source): source is Linked {
  return source in SOURCES_WITH_A_LINK;
}

/** One quiet line for each of these sources that has a result on the page; nothing when none does. */
export function ResultsCredits({ sources }: { sources: Source[] }) {
  const present = [...new Set(sources)].filter(isLinked);
  if (present.length === 0) return null;
  return (
    <div className="px-2 pt-1 text-xs text-muted-foreground">
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
