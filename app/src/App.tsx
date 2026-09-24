import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { BookPlus, CircleCheck, ScanBarcode, Settings } from "lucide-react";

import { AddBookDialog } from "@/components/add-book-dialog";
import { BookDialog } from "@/components/book-dialog";
import { BookTable } from "@/components/book-table";
import { BurstDialog } from "@/components/burst-dialog";
import { CatalogFilters, isFiltering } from "@/components/catalog-filters";
import { CatalogOptionsContext, emptyOptions } from "@/components/catalog-options";
import { SettingsDialog } from "@/components/settings-dialog";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { supportedLanguages } from "@/i18n";
import {
  type BookFilter,
  type BookSummary,
  type CatalogOptions,
  type CreatedBook,
  catalogOptions,
  listBooks,
} from "@/lib/api";

type LoadState =
  | { status: "loading" }
  | { status: "ready"; books: BookSummary[] }
  | { status: "error"; message: string };

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
  const [options, setOptions] = useState<CatalogOptions>(emptyOptions);
  const [filter, setFilter] = useState<BookFilter>({});
  const [dialogOpen, setDialogOpen] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [openBookId, setOpenBookId] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [burstOpen, setBurstOpen] = useState(false);
  const [version, setVersion] = useState(0);

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
    return () => {
      cancelled = true;
    };
  }, [activeFilter, version]);

  /** Reloads books and suggestions after any change. */
  const reload = useCallback(() => setVersion((v) => v + 1), []);

  const books = state.status === "ready" ? state.books : [];
  const filtering = isFiltering(activeFilter);

  const handleCreated = (result: CreatedBook, title: string) => {
    const message = t(
      result.existingEdition ? "catalog.addedCopy" : "catalog.added",
      { title },
    );
    setNotice(
      result.coverFailed ? `${message} ${t("catalog.coverFailed")}` : message,
    );
    reload();
  };

  const openDialog = () => {
    setNotice(null);
    setDialogOpen(true);
  };

  const catalogIsEmpty = !filtering && books.length === 0;

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
              aria-label={t("settings.title")}
              title={t("settings.title")}
              onClick={() => setSettingsOpen(true)}
            >
              <Settings />
            </Button>
            <Button variant="outline" onClick={() => setBurstOpen(true)}>
              <ScanBarcode /> {t("burst.open")}
            </Button>
            <Button onClick={openDialog}>
              <BookPlus /> {t("catalog.add")}
            </Button>
          </div>
        </header>

        {notice && (
          <Alert>
            <CircleCheck />
            <AlertDescription>{notice}</AlertDescription>
          </Alert>
        )}

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
            <Button onClick={openDialog}>
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
          <BookTable books={books} onOpen={(book) => setOpenBookId(book.editionId)} />
        )}

        <AddBookDialog
          open={dialogOpen}
          onOpenChange={setDialogOpen}
          onCreated={handleCreated}
        />
        <SettingsDialog
          open={settingsOpen}
          onOpenChange={setSettingsOpen}
          onChanged={reload}
        />
        <BurstDialog open={burstOpen} onOpenChange={setBurstOpen} onChanged={reload} />
        <BookDialog
          editionId={openBookId}
          onClose={() => setOpenBookId(null)}
          onChanged={reload}
        />
      </div>
    </CatalogOptionsContext.Provider>
  );
}
