import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, ListOrdered, Plus } from "lucide-react";

import { Button } from "@/components/ui/button";
import { type SeriesSummary, listSeries } from "@/lib/api";
import { leaveSection, openOverlay } from "@/lib/navigation";

/** The library's series (D61): name, ordered or not, how much of it is owned. Design left this
 * list undrawn; kept simple and consistent with the other list pages (library, devices…). */
export function SeriesPage({ desktop, version }: { desktop: boolean; version: number }) {
  const { t } = useTranslation();
  const [series, setSeries] = useState<SeriesSummary[] | null>(null);

  useEffect(() => {
    let cancelled = false;
    listSeries()
      .then((loaded) => !cancelled && setSeries(loaded))
      .catch(() => !cancelled && setSeries([]));
    return () => {
      cancelled = true;
    };
  }, [version]);

  return (
    <div className="flex min-h-full flex-col gap-4">
      {desktop ? (
        <div className="flex items-center gap-3">
          <h1 className="flex-1 text-[22px] font-semibold tracking-tight">{t("nav.series")}</h1>
          <Button onClick={() => openOverlay({ name: "newSeries" })}>
            <Plus /> {t("series.new")}
          </Button>
        </div>
      ) : (
        <header className="-mx-4 -mt-1 flex h-16 items-center gap-1 px-1">
          <Button
            variant="ghost"
            size="icon-lg"
            className="size-12 rounded-full"
            aria-label={t("book.back")}
            onClick={leaveSection}
          >
            <ArrowLeft className="size-[22px]" />
          </Button>
          <h1 className="flex-1 text-lg font-semibold">{t("nav.series")}</h1>
        </header>
      )}

      <div className="flex w-full max-w-2xl flex-col gap-1">
        {series?.length === 0 && (
          <p className="text-sm text-muted-foreground">{t("series.empty")}</p>
        )}
        {series?.map((s) => (
          <button
            key={s.id}
            type="button"
            className="flex min-h-16 items-center gap-3.5 rounded-xl border px-3.5 py-2 text-left hover:bg-muted"
            onClick={() => openOverlay({ name: "series", seriesId: s.id })}
          >
            <span className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted">
              <ListOrdered className="size-[18px]" />
            </span>
            <div className="flex min-w-0 flex-1 flex-col gap-0.5">
              <span className="truncate text-[15px] font-semibold">{s.name}</span>
              <span className="text-[13px] text-muted-foreground">
                {t(s.ordered ? "series.ordered" : "series.unordered")}
                {" · "}
                {t("series.haveOf", { have: s.have, total: s.total })}
              </span>
            </div>
          </button>
        ))}
        {!desktop && (
          <Button
            className="mt-2 h-12 rounded-full"
            onClick={() => openOverlay({ name: "newSeries" })}
          >
            <Plus /> {t("series.new")}
          </Button>
        )}
      </div>
    </div>
  );
}
