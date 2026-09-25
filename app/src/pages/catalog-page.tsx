import { useTranslation } from "react-i18next";
import { BookPlus, Plus } from "lucide-react";

import { BookCards } from "@/components/book-cards";
import { BookTable } from "@/components/book-table";
import { CatalogFilters } from "@/components/catalog-filters";
import { Button } from "@/components/ui/button";
import { type BookFilter, type BookSummary } from "@/lib/api";
import { openOverlay } from "@/lib/navigation";

export type LoadState =
  | { status: "loading" }
  | { status: "ready"; books: BookSummary[] }
  | { status: "error"; message: string };

export function CatalogPage({
  state,
  filter,
  onFilterChange,
  filtering,
  desktop,
}: {
  state: LoadState;
  filter: BookFilter;
  onFilterChange: (filter: BookFilter) => void;
  filtering: boolean;
  desktop: boolean;
}) {
  const { t } = useTranslation();
  const books = state.status === "ready" ? state.books : [];
  const catalogIsEmpty = !filtering && books.length === 0;
  const openBook = (book: BookSummary) => openOverlay({ name: "book", editionId: book.editionId });

  return (
    <div className="space-y-4">
      {desktop && (
        <div className="flex items-baseline gap-2.5">
          <h1 className="text-[22px] font-semibold tracking-tight">{t("nav.catalog")}</h1>
          {state.status === "ready" && (
            <span className="text-[13px] text-muted-foreground">
              {t(filtering ? "filters.results" : "catalog.count", { count: books.length })}
            </span>
          )}
        </div>
      )}
      {!catalogIsEmpty && (
        <CatalogFilters filter={filter} onChange={onFilterChange} showSearch={!desktop} />
      )}

      {state.status === "loading" && <p className="text-muted-foreground">{t("catalog.loading")}</p>}
      {state.status === "error" && (
        <p className="text-destructive">{t("errors.internal", { message: state.message })}</p>
      )}
      {state.status === "ready" && catalogIsEmpty && (
        <div className="rounded-lg border border-dashed p-12 text-center">
          <p className="font-medium">{t("catalog.empty.title")}</p>
          <p className="mb-4 text-sm text-muted-foreground">{t("catalog.empty.body")}</p>
          <Button onClick={() => openOverlay({ name: "add" })}>
            <BookPlus /> {t("catalog.add")}
          </Button>
        </div>
      )}
      {state.status === "ready" && filtering && books.length === 0 && (
        <div className="rounded-lg border border-dashed p-8 text-center">
          <p className="mb-4 text-muted-foreground">{t("filters.noResults")}</p>
          <Button variant="outline" onClick={() => onFilterChange({})}>
            {t("filters.clear")}
          </Button>
        </div>
      )}
      {state.status === "ready" && books.length > 0 &&
        (desktop ? (
          <BookTable books={books} onOpen={openBook} />
        ) : (
          <BookCards books={books} onOpen={openBook} />
        ))}

      {!desktop && (
        <Button
          className="fixed right-4 bottom-[calc(112px+env(safe-area-inset-bottom))] h-14 gap-2.5 rounded-2xl pr-5 pl-4 text-[15px] font-semibold shadow-[0_8px_20px_-6px_oklch(0_0_0/35%)]"
          onClick={() => openOverlay({ name: "add" })}
        >
          <Plus className="size-[22px]" /> {t("nav.add")}
        </Button>
      )}
    </div>
  );
}
