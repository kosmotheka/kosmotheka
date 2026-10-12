import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Gift, LibraryBig, Plus, Search, SearchX, X } from "lucide-react";

import { BookList } from "@/components/book-list";
import { BookTable } from "@/components/book-table";
import { FilterBar } from "@/components/catalog-filters";
import { useLibraries } from "@/components/libraries-context";
import { useRequests } from "@/components/requests-context";
import { SearchScope } from "@/components/search-scope";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { type BookFilter, type BookSummary, type Holding, listBooks } from "@/lib/api";
import { openOverlay } from "@/lib/navigation";
import { cn } from "@/lib/utils";

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
  holding,
  onViewChange,
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
  /** «Lo tengo · Me falta · Todo» (D61); undefined is «Todo». */
  holding: Holding | undefined;
  /** Desktop segmented control: picking «Me falta» moves to that section. */
  onViewChange: (holding: Holding | undefined) => void;
}) {
  const { t } = useTranslation();
  const reader = useRequests().myRole === "reader";
  const libraries = useLibraries();
  const missing = holding === "missing";
  const books = state.status === "ready" ? state.books : [];
  const catalogIsEmpty = state.status === "ready" && !filtering && books.length === 0;
  const openBook = (book: BookSummary) => onOpenBook(book.libraryId, book.editionId);
  const groups = everywhere ? groupByLibrary(books) : [];
  const list = (shown: BookSummary[]) =>
    desktop ? (
      <BookTable
        books={shown}
        onOpen={openBook}
        selectedId={selectedId}
        activeKind={filter.kind ?? null}
        showWish={missing}
      />
    ) : (
      <BookList
        books={shown}
        onOpen={openBook}
        activeKind={filter.kind ?? null}
        showWish={missing}
      />
    );

  // «2 de 23 · Lo que desean otros»: the wish filters need the view's total, fetched
  // separately since it excludes themselves.
  const [wishTotal, setWishTotal] = useState<number | null>(null);
  const wishFiltering = Boolean(filter.myWished || filter.othersWished);
  useEffect(() => {
    if (!wishFiltering) return;
    let cancelled = false;
    const rest: BookFilter = { ...filter, holding, myWished: undefined, othersWished: undefined };
    listBooks(rest, everywhere)
      .then((all) => !cancelled && setWishTotal(all.length))
      .catch(() => {});
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wishFiltering, holding, filter.kind, filter.query, everywhere]);

  const wishLabel =
    filter.myWished && !filter.othersWished
      ? t("filters.wish.mine")
      : filter.othersWished && !filter.myWished
        ? t("filters.wish.others")
        : t("filters.wish.chip");
  const count =
    wishFiltering && wishTotal !== null
      ? t("filters.wishCount", { count: books.length, total: wishTotal, label: wishLabel })
      : t(filtering ? "filters.results" : "nav.works", { count: books.length });

  const heading = missing ? t("nav.missing") : t("nav.catalog");
  const views: { key: Holding | undefined; label: string }[] = [
    { key: "owned", label: t("catalog.views.owned") },
    { key: "missing", label: t("nav.missing") },
    { key: undefined, label: t("catalog.views.all") },
  ];

  return (
    <div className="flex min-h-full flex-col gap-3">
      {desktop && (
        <div className="flex items-center gap-2.5">
          <h1 className="text-[22px] font-semibold tracking-tight">{heading}</h1>
          {state.status === "ready" && (
            <span className="flex-1 text-[13px] text-muted-foreground">{count}</span>
          )}
          <div
            role="radiogroup"
            aria-label={t("nav.catalog")}
            className="flex gap-0.5 rounded-[9px] bg-muted p-[3px]"
          >
            {views.map((view) => {
              const on = view.key === holding;
              return (
                <button
                  key={view.key ?? "all"}
                  type="button"
                  role="radio"
                  aria-checked={on}
                  onClick={() => onViewChange(view.key)}
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
        </div>
      )}
      {!desktop && missing && (
        <div className="flex items-baseline gap-2">
          <h1 className="text-[22px] font-semibold tracking-tight">{heading}</h1>
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
      {!catalogIsEmpty && <FilterBar filter={filter} onChange={onFilterChange} desktop={desktop} />}
      {!desktop && !missing && state.status === "ready" && !catalogIsEmpty && (
        <p className="text-xs text-muted-foreground">{count}</p>
      )}

      {state.status === "loading" && (
        <p className="text-muted-foreground">{t("catalog.loading")}</p>
      )}
      {state.status === "error" && (
        <p className="text-destructive">{t("errors.internal", { message: state.message })}</p>
      )}
      {catalogIsEmpty && missing && (
        <EmptyState icon={<Gift />} title={t("wishes.empty.title")} body={t("wishes.empty.body")}>
          <Button onClick={() => openOverlay({ name: "add", missing: true })}>
            <Plus /> {t(reader ? "ask.add" : desktop ? "nav.add" : "nav.addMissing")}
          </Button>
        </EmptyState>
      )}
      {catalogIsEmpty && !missing && (
        <EmptyState
          icon={<LibraryBig />}
          title={t("catalog.empty.title", { library })}
          body={t("catalog.empty.body")}
        >
          <Button variant="outline" onClick={() => openOverlay({ name: "backups" })}>
            {t("data.importCsv")}
          </Button>
          <Button onClick={() => openOverlay({ name: "add" })}>
            <Plus /> {t(reader ? "ask.add" : "catalog.empty.add")}
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
          <Button
            onClick={() =>
              openOverlay(
                missing
                  ? { name: "add", missing: true }
                  : { name: "add", prefill: { title: filter.query } },
              )
            }
          >
            <Plus />{" "}
            {t(reader ? "ask.add" : missing ? (desktop ? "nav.add" : "nav.addMissing") : "nav.add")}
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
          onClick={() => openOverlay(missing ? { name: "add", missing: true } : { name: "add" })}
        >
          <Plus className="size-[22px]" />{" "}
          {t(reader ? "ask.add" : missing ? "nav.addMissing" : "nav.add")}
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
