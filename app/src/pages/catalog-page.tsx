import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { LibraryBig, Plus, Search, SearchX, X } from "lucide-react";

import { BookList } from "@/components/book-list";
import { BookTable } from "@/components/book-table";
import { FilterBar } from "@/components/catalog-filters";
import { useLibraries } from "@/components/libraries-context";
import { SearchScope } from "@/components/search-scope";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { type BookFilter, type BookSummary } from "@/lib/api";
import { openOverlay } from "@/lib/navigation";

export type LoadState =
  | { status: "loading" }
  | { status: "ready"; books: BookSummary[] }
  | { status: "error"; message: string };

export function CatalogPage({
  state,
  library,
  filter,
  onFilterChange,
  filtering,
  desktop,
  searching,
  onSearchingChange,
  selectedId,
  everywhere,
  onEverywhereChange,
  nameOfLibrary,
  onOpenBook,
}: {
  state: LoadState;
  library: string;
  filter: BookFilter;
  onFilterChange: (filter: BookFilter) => void;
  filtering: boolean;
  desktop: boolean;
  /** Mobile only: the search box shows after tapping the app bar's search button. */
  searching: boolean;
  onSearchingChange: (searching: boolean) => void;
  selectedId: string | null;
  /** The search covers every library; results come grouped by library. */
  everywhere: boolean;
  onEverywhereChange: (everywhere: boolean) => void;
  nameOfLibrary: (id: string) => string;
  onOpenBook: (libraryId: string, editionId: string) => void;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const books = state.status === "ready" ? state.books : [];
  const catalogIsEmpty = state.status === "ready" && !filtering && books.length === 0;
  const openBook = (book: BookSummary) => onOpenBook(book.libraryId, book.editionId);
  const groups = everywhere ? groupByLibrary(books) : [];
  const list = (shown: BookSummary[]) =>
    desktop ? (
      <BookTable books={shown} onOpen={openBook} selectedId={selectedId} />
    ) : (
      <BookList books={shown} onOpen={openBook} />
    );
  const count = t(filtering ? "filters.results" : "nav.works", { count: books.length });

  return (
    <div className="flex min-h-full flex-col gap-3">
      {desktop && (
        <div className="flex items-baseline gap-2.5">
          <h1 className="text-[22px] font-semibold tracking-tight">{t("nav.catalog")}</h1>
          {state.status === "ready" && (
            <span className="text-[13px] text-muted-foreground">{count}</span>
          )}
        </div>
      )}
      {!desktop && (searching || filter.query) && (
        <div className="relative">
          <Search className="absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            id="catalog-search"
            enterKeyHint="search"
            autoFocus
            aria-label={t("filters.search")}
            placeholder={t("filters.searchPlaceholder")}
            className="h-11 rounded-full pr-11 pl-9"
            value={filter.query ?? ""}
            onChange={(e) => onFilterChange({ ...filter, query: e.target.value })}
          />
          <Button
            variant="ghost"
            size="icon"
            className="absolute top-1/2 right-1 -translate-y-1/2 rounded-full"
            aria-label={t("filters.closeSearch")}
            onClick={() => {
              onFilterChange({ ...filter, query: undefined });
              onSearchingChange(false);
            }}
          >
            <X />
          </Button>
        </div>
      )}
      {!desktop && filter.query?.trim() && libraries.all.length > 1 && (
        <SearchScope library={library} everywhere={everywhere} onChange={onEverywhereChange} />
      )}
      {!catalogIsEmpty && <FilterBar filter={filter} onChange={onFilterChange} />}
      {!desktop && state.status === "ready" && !catalogIsEmpty && (
        <p className="text-xs text-muted-foreground">{count}</p>
      )}

      {state.status === "loading" && <p className="text-muted-foreground">{t("catalog.loading")}</p>}
      {state.status === "error" && (
        <p className="text-destructive">{t("errors.internal", { message: state.message })}</p>
      )}
      {catalogIsEmpty && (
        <EmptyState
          icon={<LibraryBig />}
          title={t("catalog.empty.title", { library })}
          body={t("catalog.empty.body")}
        >
          <Button variant="outline" onClick={() => openOverlay({ name: "backups" })}>
            {t("data.importCsv")}
          </Button>
          <Button onClick={() => openOverlay({ name: "add" })}>
            <Plus /> {t("catalog.empty.add")}
          </Button>
        </EmptyState>
      )}
      {state.status === "ready" && filtering && books.length === 0 && (
        <EmptyState
          icon={<SearchX />}
          title={t("filters.noResultsTitle")}
          body={t("filters.noResults")}
        >
          <Button variant="outline" onClick={() => onFilterChange({})}>
            {t("filters.clear")}
          </Button>
          <Button onClick={() => openOverlay({ name: "add", prefill: { title: filter.query } })}>
            <Plus /> {t("catalog.add")}
          </Button>
        </EmptyState>
      )}
      {state.status === "ready" && books.length > 0 && (
        // On mobile, room for the floating "Add" button over the last row.
        <div className={desktop ? undefined : "pb-16"}>
          {everywhere
            ? groups.map(([libraryId, shown]) => (
                <section key={libraryId} className="mt-2 first:mt-0">
                  <h2 className="flex items-center gap-2 border-b py-2.5 text-[13px] font-semibold">
                    <LibraryBig className="size-[15px] text-muted-foreground" />
                    {nameOfLibrary(libraryId)}
                    <span className="font-normal text-muted-foreground">{shown.length}</span>
                    {libraryId !== libraries.active?.id && (
                      <span className="ml-auto text-xs font-normal text-muted-foreground">
                        {t("libraries.opensThere", { name: nameOfLibrary(libraryId) })}
                      </span>
                    )}
                  </h2>
                  {list(shown)}
                </section>
              ))
            : list(books)}
        </div>
      )}

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

/** Books by library, in the order they came (the backend lists library by library). */
function groupByLibrary(books: BookSummary[]): [string, BookSummary[]][] {
  const groups = new Map<string, BookSummary[]>();
  for (const book of books) {
    groups.set(book.libraryId, [...(groups.get(book.libraryId) ?? []), book]);
  }
  return [...groups];
}

function EmptyState({
  icon,
  title,
  body,
  children,
}: {
  icon: ReactNode;
  title: string;
  body: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-2.5 pt-10 pb-20 text-center">
      <div className="flex size-13 items-center justify-center rounded-full bg-muted text-muted-foreground [&_svg]:size-6">
        {icon}
      </div>
      <p className="mt-1 text-[17px] font-semibold">{title}</p>
      <p className="max-w-[420px] text-sm text-pretty text-muted-foreground">{body}</p>
      <div className="mt-2 flex flex-wrap justify-center gap-2">{children}</div>
    </div>
  );
}
