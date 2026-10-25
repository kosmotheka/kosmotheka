import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, Layers, ListOrdered, Plus } from "lucide-react";

import { type CatalogView, CatalogViews } from "@/components/catalog-views";
import { Cover } from "@/components/cover";
import { Button } from "@/components/ui/button";
import { type SeriesSummary, listSeries } from "@/lib/api";
import { leaveSection, openOverlay } from "@/lib/navigation";

/**
 * «Sagas y colecciones» (D61), a view of the catalogue: works that go together, a saga in order (Dune 1, 2, 3…) or
 * a collection without one (Catan and its expansions), and how much of each you have. It says what it is for,
 * since the word alone did not, and an empty one says how to start.
 */
export function SeriesPage({
  desktop,
  version,
  onViewChange,
}: {
  desktop: boolean;
  version: number;
  onViewChange: (view: CatalogView) => void;
}) {
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

  const newButton = (
    <Button
      className={desktop ? undefined : "h-12 rounded-full"}
      onClick={() => openOverlay({ name: "newSeries" })}
    >
      <Plus /> {t("series.new")}
    </Button>
  );

  const kinds = (
    <div className="grid gap-2 sm:grid-cols-2">
      <Explainer icon={<ListOrdered className="size-[18px]" />} title={t("series.orderedKind")}>
        {t("series.orderedExplain")}
      </Explainer>
      <Explainer icon={<Layers className="size-[18px]" />} title={t("series.unorderedKind")}>
        {t("series.unorderedExplain")}
      </Explainer>
    </div>
  );

  return (
    <div className="flex min-h-full flex-col gap-4">
      {desktop ? (
        <div className="flex items-center gap-2.5">
          <h1 className="flex-1 text-[22px] font-semibold tracking-tight">{t("nav.series")}</h1>
          <CatalogViews active="series" onChange={onViewChange} />
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

      <div className="flex w-full max-w-2xl flex-col gap-3">
        <p className="text-[15px] text-pretty">{t("series.intro")}</p>

        {series?.length === 0 && (
          <>
            {kinds}
            <div className="flex flex-col gap-1.5 rounded-[14px] border border-dashed p-4">
              <p className="font-semibold">{t("series.emptyTitle")}</p>
              <p className="text-sm text-pretty text-muted-foreground">{t("series.emptyHow")}</p>
            </div>
          </>
        )}

        {series && series.length > 0 && (
          <div className="flex flex-col gap-2">
            {series.map((s) => (
              <SeriesRow key={s.id} series={s} />
            ))}
          </div>
        )}

        <div className={desktop ? "flex" : "flex flex-col"}>{newButton}</div>

        {series && series.length > 0 && (
          <details className="text-sm">
            <summary className="cursor-pointer text-muted-foreground">
              {t("series.whatAreThey")}
            </summary>
            <div className="flex flex-col gap-2 pt-2">
              {kinds}
              <p className="text-pretty text-muted-foreground">{t("series.emptyHow")}</p>
            </div>
          </details>
        )}
      </div>
    </div>
  );
}

function Explainer({
  icon,
  title,
  children,
}: {
  icon: ReactNode;
  title: string;
  children: ReactNode;
}) {
  return (
    <div className="flex gap-3 rounded-[14px] border p-3">
      <span className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted">
        {icon}
      </span>
      <div className="flex min-w-0 flex-col gap-0.5">
        <span className="text-sm font-semibold">{title}</span>
        <span className="text-[13px] text-pretty text-muted-foreground">{children}</span>
      </div>
    </div>
  );
}

function SeriesRow({ series: s }: { series: SeriesSummary }) {
  const { t } = useTranslation();
  const done = s.total > 0 ? s.have / s.total : 0;
  return (
    <button
      type="button"
      className="flex items-center gap-3.5 rounded-xl border p-3 text-left hover:bg-muted"
      onClick={() => openOverlay({ name: "series", seriesId: s.id })}
    >
      <Covers series={s} />
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="truncate text-[15px] font-semibold">{s.name}</span>
        <span className="text-[13px] text-muted-foreground">
          {t(s.ordered ? "series.orderedKind" : "series.unorderedKind")}
          {" · "}
          {t("series.haveOf", { have: s.have, total: s.total })}
          {s.missing > 0 && <> · {t("series.missing", { count: s.missing })}</>}
        </span>
        <div
          className="h-1.5 overflow-hidden rounded-full bg-muted"
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={s.total}
          aria-valuenow={s.have}
          aria-label={t("series.haveOf", { have: s.have, total: s.total })}
        >
          <div className="h-full bg-primary" style={{ width: `${done * 100}%` }} />
        </div>
      </div>
    </button>
  );
}

/** Up to three covers, fanned out; the kind's icon when there are none. */
function Covers({ series: s }: { series: SeriesSummary }) {
  if (s.covers.length === 0) {
    return (
      <span className="flex h-14 w-10 shrink-0 items-center justify-center rounded-md bg-muted">
        {s.ordered ? <ListOrdered className="size-[18px]" /> : <Layers className="size-[18px]" />}
      </span>
    );
  }
  return (
    <span
      className="relative h-14 shrink-0"
      style={{ width: `${40 + (s.covers.length - 1) * 12}px` }}
    >
      {s.covers.map((cover, i) => (
        <span
          key={cover.file}
          className="absolute top-0 rounded-md ring-2 ring-background"
          style={{ left: `${i * 12}px`, zIndex: s.covers.length - i }}
        >
          <Cover file={cover.file} alt="" kind={cover.kind} className="h-14" />
        </span>
      ))}
    </span>
  );
}
