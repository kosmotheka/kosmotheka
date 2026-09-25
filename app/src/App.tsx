import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { CircleCheck } from "lucide-react";

import { AddBookDialog } from "@/components/add-book-dialog";
import { BookDialog } from "@/components/book-dialog";
import { BurstDialog } from "@/components/burst-dialog";
import { isFiltering } from "@/components/catalog-filters";
import { CatalogOptionsContext, emptyOptions } from "@/components/catalog-options";
import { CheckDialog } from "@/components/check-dialog";
import { DevicesDialog } from "@/components/devices-dialog";
import { SettingsDialog } from "@/components/settings-dialog";
import { AppBar } from "@/components/shell/app-bar";
import { BottomNav } from "@/components/shell/bottom-nav";
import { LibraryPicker } from "@/components/shell/library-picker";
import { NavDrawer } from "@/components/shell/nav-drawer";
import { Sidebar } from "@/components/shell/sidebar";
import { useSyncStatus } from "@/components/shell/sync-status";
import { TopBar } from "@/components/shell/top-bar";
import { useDesktop } from "@/components/shell/use-desktop";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  type BookFilter,
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
import { closeOverlay, navigate, openOverlay, useLocation } from "@/lib/navigation";
import { type LoadState, CatalogPage } from "@/pages/catalog-page";
import { HelpPage } from "@/pages/help-page";
import { MissingPage } from "@/pages/missing-page";

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
  const { t } = useTranslation();
  const { section, overlay } = useLocation();
  const desktop = useDesktop();
  const [state, setState] = useState<LoadState>({ status: "loading" });
  const [wishes, setWishes] = useState<Wish[]>([]);
  const [options, setOptions] = useState<CatalogOptions>(emptyOptions);
  const [filter, setFilter] = useState<BookFilter>({});
  const [notice, setNotice] = useState<string | null>(null);
  const [version, setVersion] = useState(0);
  const sync = useSyncStatus(version);

  // Until several libraries exist (roadmap phase 4), there is only this one.
  const library = t("nav.myLibrary");

  const activeFilter = useDebounced(filter, 200);

  useEffect(() => {
    let cancelled = false;
    listBooks(activeFilter)
      .then((books) => !cancelled && setState({ status: "ready", books }))
      .catch(
        (error: unknown) => !cancelled && setState({ status: "error", message: String(error) }),
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

  // Quietly catch up with every reachable paired device when the app opens, and again
  // when it comes back to the foreground (a phone arriving home), at most once a minute.
  useEffect(() => {
    let last = 0;
    let running = false;
    const syncAll = async () => {
      if (running || Date.now() - last < 60_000) return;
      running = true;
      last = Date.now();
      const status = await lanStatus().catch(() => null);
      for (const device of status?.servers ?? []) {
        await syncWith(device.deviceId).catch(() => null);
      }
      running = false;
      // Reload even without changes: the sync indicator shows the new time.
      reload();
    };
    const onVisible = () => {
      if (document.visibilityState === "visible") void syncAll();
    };
    void syncAll();
    document.addEventListener("visibilitychange", onVisible);
    return () => document.removeEventListener("visibilitychange", onVisible);
  }, [reload]);

  const books = state.status === "ready" ? state.books : [];
  const filtering = isFiltering(activeFilter);
  const counts = { catalog: filtering ? 0 : books.length, missing: wishes.length };

  const handleCreated = async (result: CreatedBook, title: string, wishId?: string) => {
    const message = t(result.existingEdition ? "catalog.addedCopy" : "catalog.added", { title });
    setNotice(result.coverFailed ? `${message} ${t("catalog.coverFailed")}` : message);
    // A wish that became a book is no longer wished for.
    if (wishId) await deleteWish(wishId).catch(() => {});
    reload();
  };

  const focusSearch = () => {
    navigate("catalog");
    setTimeout(() => document.getElementById("catalog-search")?.focus(), 50);
  };

  const page =
    section === "missing" ? (
      <MissingPage wishes={wishes} desktop={desktop} onChanged={reload} />
    ) : section === "help" ? (
      <HelpPage />
    ) : (
      <CatalogPage
        state={state}
        filter={filter}
        onFilterChange={setFilter}
        filtering={filtering}
        desktop={desktop}
      />
    );

  const content = (
    <>
      {notice && (
        <Alert className="mb-4">
          <CircleCheck />
          <AlertDescription>{notice}</AlertDescription>
        </Alert>
      )}
      {page}
    </>
  );

  return (
    <CatalogOptionsContext.Provider value={options}>
      {desktop ? (
        <div className="flex h-dvh">
          <Sidebar section={section} library={library} counts={counts} />
          <div className="flex min-w-0 flex-1 flex-col">
            <TopBar
              library={library}
              query={filter.query ?? ""}
              onQueryChange={(query) => setFilter({ ...filter, query })}
              sync={sync}
            />
            <main className="flex-1 overflow-y-auto px-6 pt-5 pb-8">{content}</main>
          </div>
        </div>
      ) : (
        <div className="flex h-dvh flex-col pt-[env(safe-area-inset-top)]">
          <AppBar library={library} sync={sync} onSearch={focusSearch} />
          <main className="flex-1 overflow-y-auto px-4 pt-1 pb-24">{content}</main>
          <BottomNav section={section} />
          <NavDrawer
            open={overlay?.name === "drawer"}
            library={library}
            count={counts.catalog}
            sync={sync}
          />
        </div>
      )}

      <LibraryPicker
        open={overlay?.name === "libraries"}
        desktop={desktop}
        library={library}
        count={counts.catalog}
      />
      {overlay?.name === "add" && (
        <AddBookDialog
          open
          prefill={overlay.prefill}
          onOpenChange={(open) => !open && closeOverlay()}
          onCreated={(result, title) => void handleCreated(result, title, overlay.wishId)}
        />
      )}
      <SettingsDialog
        open={overlay?.name === "settings"}
        onOpenChange={(open) => !open && closeOverlay()}
        onChanged={reload}
      />
      <BurstDialog
        open={overlay?.name === "burst"}
        onOpenChange={(open) => !open && closeOverlay()}
        onChanged={reload}
      />
      <DevicesDialog
        open={overlay?.name === "devices"}
        onOpenChange={(open) => !open && closeOverlay()}
        onChanged={reload}
      />
      <CheckDialog
        open={overlay?.name === "check"}
        onOpenChange={(open) => !open && closeOverlay()}
        onOpenBook={(editionId) => openOverlay({ name: "book", editionId })}
        onAddToCatalog={(prefill) => openOverlay({ name: "add", prefill })}
        onChanged={reload}
      />
      <BookDialog
        editionId={overlay?.name === "book" ? overlay.editionId : null}
        onClose={closeOverlay}
        onChanged={reload}
      />
    </CatalogOptionsContext.Provider>
  );
}
