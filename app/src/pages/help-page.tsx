import { Trans, useTranslation } from "react-i18next";

import { ExternalLink } from "@/components/external-link";
import { Button } from "@/components/ui/button";
import { guidePage } from "@/lib/guide";
import { ChevronDown } from "lucide-react";

const TOPICS = [
  "search",
  "share",
  "pairing",
  "away",
  "cloud",
  "people",
  "trash",
  "backups",
  "lost",
] as const;

/** The guide's page that tells each topic in full, the same path in every language. */
const PAGE: Record<(typeof TOPICS)[number], string> = {
  search: "add-a-book/",
  share: "share-at-home/",
  pairing: "share-at-home/",
  away: "away-from-home/",
  cloud: "the-cloud/",
  people: "people-and-permissions/",
  trash: "trash/",
  backups: "backups/",
  lost: "lost-phone/",
};

/** Short answers, one folded topic each, with its page of the full guide. */
export function HelpPage({
  desktop,
  onShowWelcome,
}: {
  desktop: boolean;
  onShowWelcome: () => void;
}) {
  const { t, i18n } = useTranslation();
  const guide = guidePage(i18n.language);
  return (
    <div className="flex max-w-3xl flex-col gap-3">
      {desktop && <h1 className="text-[22px] font-semibold tracking-tight">{t("help.title")}</h1>}
      <p className="text-sm text-muted-foreground">{t("help.intro")}</p>
      <div className="-mx-4 flex flex-col md:mx-0">
        {TOPICS.map((topic) => (
          <details key={topic} className="group border-b last:border-b-0">
            <summary className="flex min-h-14 cursor-pointer list-none items-center gap-3 px-4 text-[15px] font-medium md:px-1 [&::-webkit-details-marker]:hidden">
              <span className="flex-1">{t(`help.topics.${topic}.title`)}</span>
              <ChevronDown className="size-5 text-muted-foreground transition-transform group-open:rotate-180" />
            </summary>
            <ol className="flex list-decimal flex-col gap-2 pr-4 pb-4 pl-9 text-sm md:pr-1 md:pl-6">
              {(t(`help.topics.${topic}.steps`, { returnObjects: true }) as string[]).map(
                (step) => (
                  <li key={step}>{step}</li>
                ),
              )}
            </ol>
            <p className="pr-4 pb-4 pl-9 text-sm md:pr-1 md:pl-6">
              <ExternalLink url={guide + PAGE[topic]}>{t("help.more")}</ExternalLink>
            </p>
          </details>
        ))}
      </div>
      <p className="text-sm text-muted-foreground">
        <Trans i18nKey="help.fullGuide" components={{ link: <ExternalLink url={guide} /> }} />
      </p>
      <Button variant="outline" className="self-start" onClick={onShowWelcome}>
        {t("help.showWelcome")}
      </Button>
    </div>
  );
}
