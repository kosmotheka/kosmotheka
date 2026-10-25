import { useTranslation } from "react-i18next";
import { CircleAlert } from "lucide-react";

import { Button } from "@/components/ui/button";
import type { Source } from "@/lib/api";
import { openOverlay } from "@/lib/navigation";

export interface ProviderWarning {
  source: Source;
  kind: "network" | "quota" | "bad_key" | "unexpected";
}

/**
 * What failed in a search, next to its results, until the next search (D83): a catalogue that did not answer can
 * be why something is missing. With a way to Providers, where a key fixes a quota or a bad key.
 */
export function ProviderWarnings({ warnings }: { warnings: ProviderWarning[] }) {
  const { t } = useTranslation();
  if (warnings.length === 0) return null;
  const fixable = warnings.some((w) => w.kind === "quota" || w.kind === "bad_key");
  return (
    <div role="status" className="flex gap-2.5 rounded-[14px] border p-3 text-[13px] leading-snug">
      <CircleAlert className="mt-px size-[18px] shrink-0 text-amber-600" />
      <div className="flex min-w-0 flex-1 flex-col items-start gap-1">
        {warnings.map((w) => (
          <p key={`${w.source}-${w.kind}`}>
            {w.kind === "quota" && w.source === "google_books"
              ? t("lookup.googleQuota")
              : w.kind === "bad_key"
                ? t("lookup.badKey", { source: t(`lookup.sources.${w.source}`) })
                : t("lookup.unreachable", { source: t(`lookup.sources.${w.source}`) })}
          </p>
        ))}
        {fixable && (
          <Button
            variant="link"
            className="h-auto p-0 text-[13px]"
            onClick={() => openOverlay({ name: "providers" })}
          >
            {t("lookup.openProviders")}
          </Button>
        )}
      </div>
    </div>
  );
}
