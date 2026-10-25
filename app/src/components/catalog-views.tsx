import { useTranslation } from "react-i18next";

import type { Holding } from "@/lib/api";
import { cn } from "@/lib/utils";

/** What the catalogue shows: works by holding (D61), or its sagas and collections. */
export type CatalogView = Holding | "all" | "series";

/** The desktop segmented control over the catalogue: «Lo tengo · Me falta · Todo · Sagas y colecciones». */
export function CatalogViews({
  active,
  onChange,
}: {
  active: CatalogView;
  onChange: (view: CatalogView) => void;
}) {
  const { t } = useTranslation();
  const views: { key: CatalogView; label: string }[] = [
    { key: "owned", label: t("catalog.views.owned") },
    { key: "missing", label: t("nav.missing") },
    { key: "all", label: t("catalog.views.all") },
    { key: "series", label: t("nav.series") },
  ];
  return (
    <div
      role="radiogroup"
      aria-label={t("nav.catalog")}
      className="flex gap-0.5 rounded-[9px] bg-muted p-[3px]"
    >
      {views.map((view) => {
        const on = view.key === active;
        return (
          <button
            key={view.key}
            type="button"
            role="radio"
            aria-checked={on}
            onClick={() => onChange(view.key)}
            className={cn(
              "h-7 rounded-[6px] px-3 text-[13px]",
              on
                ? "bg-background font-semibold text-foreground shadow-[0_1px_2px_oklch(0_0_0/12%)]"
                : "font-medium text-muted-foreground",
            )}
          >
            {view.label}
          </button>
        );
      })}
    </div>
  );
}
