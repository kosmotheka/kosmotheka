import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  BookPlus,
  CircleCheck,
  MonitorSmartphone,
  ScanBarcode,
  ScanSearch,
  Settings,
} from "lucide-react";

import { type AddBookPrefill, AddBookDialog } from "@/components/add-book-dialog";
import { BookCards } from "@/components/book-cards";
import { BookDialog } from "@/components/book-dialog";
import { BookTable } from "@/components/book-table";
import { BurstDialog } from "@/components/burst-dialog";
import { CatalogFilters, isFiltering } from "@/components/catalog-filters";
import { CatalogOptionsContext, emptyOptions } from "@/components/catalog-options";
import { CheckDialog } from "@/components/check-dialog";
import { DevicesDialog } from "@/components/devices-dialog";
import { SettingsDialog } from "@/components/settings-dialog";
import { WishList } from "@/components/wish-list";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { supportedLanguages } from "@/i18n";
import {
  type BookFilter,
  type BookSummary,
  type CatalogOptions,
  type CreatedBook,
  type Wish,
  catalogOptions,
  deleteWish,
  lanStatus,
  listBooks,
  listWishes,
  syncWith,
} from "@/lib/api";

type LoadState =
  | { status: "loading" }
  | { status: "ready"; books: BookSummary[] }
  | { status: "error"; message: string };

type View = "catalog" | "wishes";

/** Waits until typing pauses before searching. */
function useDebounced<T>(value: T, delay: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), delay);
    return () => clearTimeout(timer);
  }, [value, delay]);
  return debounced;
}

export default function App() {
  const { t, i18n } = useTranslation();
  const [state, setState] = useState<LoadState>({ status: "loading" });
  const [wishes, setWishes] = useState<Wish[]>([]);
  const [options, setOptions] = useState<CatalogOptions>(emptyOptions);
  const [filter, setFilter] = useState<BookFilter>({});
  const [view, setView] = useState<View>("catalog");
  const [notice, setNotice] = useState<string | null>(null);
  const [openBookId, setOpenBookId] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [burstOpen, setBurstOpen] = useState(false);
  const [devicesOpen, setDevicesOpen] = useState(false);
  const [checkOpen, setCheckOpen] = useState(false);
  const [version, setVersion] = useState(0);
  /** A new key remounts the add dialog so its prefill is read again. */
  const [adding, setAdding] = useState<{ key: number; prefill?: AddBookPrefill; wishId?: string } | null>(
    null,
  );

  const activeFilter = useDebounced(filter, 200);

  useEffect(() => {
    let cancelled = false;
    listBooks(activeFilter)
      .then((books) => !cancelled && setState({ status: "ready", books }))
      .catch(
        (error: unknown) =>
          !cancelled && setState({ status: "error", message: String(error) }),
      );
    catalogOptions()
      .then((loaded) => !cancelled && setOptions(loaded))
      .catch(() => {});
    listWishes()
      .then((loaded) => !cancelled && setWishes(loaded))
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [activeFilter, version]);

  /** Reloads books, wishes and suggestions after any change. */
  const reload = useCallback(() => setVersion((v) => v + 1), []);

  // On opening, quietly catch up with every paired device that is reachable.
  useEffect(() => {
    void (async () => {
      const status = await lanStatus().catch(() => null);
      let changed = false;
      for (const device of status?.servers ?? []) {
        const report = await syncWith(device.deviceId).catch(() => null);
        changed ||= (report?.received.received ?? 0) > 0;
      }
      if (changed) reload();
    })();
  }, [reload]);

  const books = state.status === "ready" ? state.books : [];
  const filtering = isFiltering(activeFilter);

  const startAdding = (prefill?: AddBookPrefill, wishId?: string) => {
    setNotice(null);
    setAdding((current) => ({ key: (current?.key ?? 0) + 1, prefill, wishId }));
  };

  const handleCreated = async (result: CreatedBook, title: string) => {
    const message = t(
      result.existingEdition ? "catalog.addedCopy" : "catalog.added",
      { title },
    );
    setNotice(
      result.coverFailed ? `${message} ${t("catalog.coverFailed")}` : message,
    );
    // A wish that became a book is no longer wished for.
    if (adding?.wishId) await deleteWish(adding.wishId).catch(() => {});
    reload();
  };

  const catalogIsEmpty = !filtering && books.length === 0;
  const label = (text: string) => <span className="hidden sm:inline">{text}</span>;

  return (
    <CatalogOptionsContext.Provider value={options}>
      <div className="mx-auto max-w-5xl space-y-6 p-4 md:p-8">
        <header className="flex flex-wrap items-center justify-between gap-4">
          <div>
            <h1 className="text-2xl font-semibold">{t("app.title")}</h1>
            {state.status === "ready" && (
              <p className="text-sm text-muted-foreground">
                {t(filtering ? "filters.results" : "catalog.count", {
                  count: books.length,
                })}
              </p>
            )}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            {supportedLanguages.map((language) => (
              <Button
                key={language}
                size="sm"
                variant={i18n.resolvedLanguage === language ? "secondary" : "ghost"}
                onClick={() => void i18n.changeLanguage(language)}
              >
                {language.toUpperCase()}
              </Button>
            ))}
            <Button
              variant="ghost"
              size="icon"
              aria-label={t("devices.title")}
              title={t("devices.title")}
              onClick={() => setDevicesOpen(true)}
            >
              <MonitorSmartphone />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              aria-label={t("settings.title")}
              title={t("settings.title")}
              onClick={() => setSettingsOpen(true)}
            >
              <Settings />
            </Button>
            <Button variant="outline" onClick={() => setCheckOpen(true)} title={t("check.title")}>
              <ScanSearch /> {label(t("check.open"))}
            </Button>
            <Button variant="outline" onClick={() => setBurstOpen(true)} title={t("burst.open")}>
              <ScanBarcode /> {label(t("burst.open"))}
            </Button>
            <Button onClick={() => startAdding()} title={t("catalog.add")}>
              <BookPlus /> {label(t("catalog.add"))}
            </Button>
          </div>
        </header>

        {notice && (
          <Alert>
            <CircleCheck />
            <AlertDescription>{notice}</AlertDescription>
          </Alert>
        )}

        <Tabs value={view} onValueChange={(value) => setView(value as View)}>
          <TabsList>
            <TabsTrigger value="catalog">{t("tabs.catalog")}</TabsTrigger>
            <TabsTrigger value="wishes">
              {t("tabs.wishes", { count: wishes.length })}
            </TabsTrigger>
          </TabsList>
        </Tabs>

        {view === "wishes" && (
          <WishList
            wishes={wishes}
            onRemove={(wish) => void deleteWish(wish.id).then(reload)}
            onBought={(wish) =>
              startAdding(
                {
                  isbn: wish.isbn13 ?? undefined,
                  title: wish.title,
                  author: wish.author ?? undefined,
                },
                wish.id,
              )
            }
          />
        )}

        {view === "catalog" && (
          <>
            {!catalogIsEmpty && <CatalogFilters filter={filter} onChange={setFilter} />}

            {state.status === "loading" && (
              <p className="text-muted-foreground">{t("catalog.loading")}</p>
            )}
            {state.status === "error" && (
              <p className="text-destructive">
                {t("errors.internal", { message: state.message })}
              </p>
            )}
            {state.status === "ready" && catalogIsEmpty && (
              <div className="rounded-lg border border-dashed p-12 text-center">
                <p className="font-medium">{t("catalog.empty.title")}</p>
                <p className="mb-4 text-sm text-muted-foreground">
                  {t("catalog.empty.body")}
                </p>
                <Button onClick={() => startAdding()}>
                  <BookPlus /> {t("catalog.add")}
                </Button>
              </div>
            )}
            {state.status === "ready" && filtering && books.length === 0 && (
              <div className="rounded-lg border border-dashed p-8 text-center">
                <p className="mb-4 text-muted-foreground">{t("filters.noResults")}</p>
                <Button variant="outline" onClick={() => setFilter({})}>
                  {t("filters.clear")}
                </Button>
              </div>
            )}
            {state.status === "ready" && books.length > 0 && (
              <>
                <div className="hidden md:block">
                  <BookTable books={books} onOpen={(book) => setOpenBookId(book.editionId)} />
                </div>
                <div className="md:hidden">
                  <BookCards books={books} onOpen={(book) => setOpenBookId(book.editionId)} />
                </div>
              </>
            )}
          </>
        )}

        {adding && (
          <AddBookDialog
            key={adding.key}
            open
            prefill={adding.prefill}
            onOpenChange={(open) => !open && setAdding(null)}
            onCreated={(result, title) => void handleCreated(result, title)}
          />
        )}
        <SettingsDialog
          open={settingsOpen}
          onOpenChange={setSettingsOpen}
          onChanged={reload}
        />
        <BurstDialog open={burstOpen} onOpenChange={setBurstOpen} onChanged={reload} />
        <DevicesDialog open={devicesOpen} onOpenChange={setDevicesOpen} onChanged={reload} />
        <CheckDialog
          open={checkOpen}
          onOpenChange={setCheckOpen}
          onOpenBook={(editionId) => {
            setCheckOpen(false);
            setOpenBookId(editionId);
          }}
          onAddToCatalog={(prefill) => {
            setCheckOpen(false);
            startAdding(prefill);
          }}
          onChanged={reload}
        />
        <BookDialog
          editionId={openBookId}
          onClose={() => setOpenBookId(null)}
          onChanged={reload}
        />
      </div>
    </CatalogOptionsContext.Provider>
  );
}
