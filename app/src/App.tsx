import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { CircleCheck } from "lucide-react";

import { AddBookDialog } from "@/components/add-book-dialog";
import { BackupsDialog } from "@/components/backups-dialog";
import { BookDetailView } from "@/components/book-detail";
import { BurstDialog } from "@/components/burst-dialog";
import { FilterPanel, isFiltering } from "@/components/catalog-filters";
import { CatalogOptionsContext, emptyOptions } from "@/components/catalog-options";
import { MoveBookDialog } from "@/components/libraries/move-book-dialog";
import { NewLibraryDialog } from "@/components/libraries/new-library-dialog";
import {
  DeleteLibraryDialog,
  LeaveLibraryDialog,
} from "@/components/libraries/remove-library-dialogs";
import { type Libraries, LibrariesContext } from "@/components/libraries-context";
import { PairDialog } from "@/components/pair-dialog";
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
  type LibrarySummary,
  type Wish,
  catalogOptions,
  deleteWish,
  lanStatus,
  listBooks,
  listLibraries,
  listWishes,
  setActiveLibrary,
} from "@/lib/api";
import { closeOverlay, navigate, openOverlay, useLocation } from "@/lib/navigation";
import { syncDevice } from "@/lib/sync-errors";
import { type LoadState, CatalogPage } from "@/pages/catalog-page";
import { CheckPage } from "@/pages/check-page";
import { DevicesPage } from "@/pages/devices-page";
import { HelpPage } from "@/pages/help-page";
import { LibraryPage } from "@/pages/library-page";
import { MissingPage } from "@/pages/missing-page";
import { PeoplePage } from "@/pages/people-page";

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
  const [searching, setSearching] = useState(false);
  const [everywhere, setEverywhere] = useState(false);
  const [allLibraries, setAllLibraries] = useState<LibrarySummary[]>([]);
  const sync = useSyncStatus(version);

  const activeLibrary = allLibraries.find((l) => l.active) ?? null;
  const library = activeLibrary?.name || t("nav.myLibrary");
  const nameOfId = (id: string) =>
    allLibraries.find((l) => l.id === id)?.name || t("nav.myLibrary");

  const activeFilter = useDebounced(filter, 200);
  // "In every library" only means something while searching.
  const searchEverywhere = everywhere && Boolean(activeFilter.query?.trim());

  useEffect(() => {
    let cancelled = false;
    listLibraries()
      .then((loaded) => !cancelled && setAllLibraries(loaded))
      .catch(() => {});
    listBooks(activeFilter, searchEverywhere)
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
  }, [activeFilter, searchEverywhere, version]);

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
      let removed = 0;
      for (const device of status?.servers ?? []) {
        const report = await syncDevice(device.deviceId).catch(() => null);
        removed += report?.removed.length ?? 0;
      }
      running = false;
      if (removed > 0) setNotice(t("sharing.removed", { count: removed }));
      // Reload even without changes: the sync indicator shows the new time.
      reload();
    };
    const onVisible = () => {
      if (document.visibilityState === "visible") void syncAll();
    };
    void syncAll();
    document.addEventListener("visibilitychange", onVisible);
    return () => document.removeEventListener("visibilitychange", onVisible);
  }, [reload, t]);

  const books = state.status === "ready" ? state.books : [];
  const filtering = isFiltering(activeFilter);
  const counts = {
    catalog: activeLibrary?.works ?? (filtering ? 0 : books.length),
    missing: wishes.length,
  };

  const handleCreated = async (result: CreatedBook, title: string, wishId?: string) => {
    const message = t(result.existingEdition ? "catalog.addedCopy" : "catalog.added", { title });
    setNotice(result.coverFailed ? `${message} ${t("catalog.coverFailed")}` : message);
    // A wish that became a book is no longer wished for.
    if (wishId) await deleteWish(wishId).catch(() => {});
    reload();
  };

  const bookId = overlay?.name === "book" ? overlay.editionId : null;

  const switchTo = async (id: string) => {
    await setActiveLibrary(id);
    reload();
  };
  const libraries: Libraries = { all: allLibraries, active: activeLibrary, switchTo, reload };

  /** Books found in another library open there: the app switches to it first. */
  const openBookIn = async (libraryId: string, editionId: string) => {
    if (libraryId !== activeLibrary?.id) await switchTo(libraryId);
    openOverlay({ name: "book", editionId });
  };

  const focusSearch = () => {
    navigate("catalog");
    setSearching(true);
    setTimeout(() => document.getElementById("catalog-search")?.focus(), 50);
  };

  const page =
    section === "missing" ? (
      <MissingPage wishes={wishes} desktop={desktop} onChanged={reload} />
    ) : section === "check" ? (
      <CheckPage desktop={desktop} onChanged={reload} onOpenBook={openBookIn} />
    ) : section === "library" ? (
      <LibraryPage desktop={desktop} />
    ) : section === "devices" ? (
      <DevicesPage desktop={desktop} version={version} onChanged={reload} onNotice={setNotice} />
    ) : section === "people" ? (
      <PeoplePage desktop={desktop} version={version} onNotice={setNotice} />
    ) : section === "help" ? (
      <HelpPage desktop={desktop} />
    ) : (
      <CatalogPage
        state={state}
        library={library}
        filter={filter}
        onFilterChange={setFilter}
        filtering={filtering}
        desktop={desktop}
        searching={searching}
        onSearchingChange={setSearching}
        everywhere={searchEverywhere}
        onEverywhereChange={setEverywhere}
        nameOfLibrary={nameOfId}
        onOpenBook={openBookIn}
        selectedId={desktop ? bookId : null}
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
    <LibrariesContext.Provider value={libraries}>
      <CatalogOptionsContext.Provider value={options}>
        {desktop ? (
          <div className="flex h-dvh">
            <Sidebar section={section} library={library} counts={counts} />
            <div className="flex min-w-0 flex-1 flex-col">
              <TopBar
                library={library}
                query={filter.query ?? ""}
                everywhere={everywhere}
                onEverywhereChange={setEverywhere}
                onQueryChange={(query) => {
                  setFilter({ ...filter, query });
                  if (section !== "catalog") navigate("catalog");
                }}
                sync={sync}
              />
              <div className="flex min-h-0 flex-1">
                <main className="min-w-0 flex-1 overflow-y-auto px-6 pt-5 pb-8">{content}</main>
                <BookDetailView
                  editionId={bookId}
                  desktop
                  onClose={closeOverlay}
                  onChanged={reload}
                />
              </div>
            </div>
          </div>
        ) : (
          <div className="flex h-dvh flex-col pt-[env(safe-area-inset-top)]">
            {section !== "check" && section !== "library" && section !== "devices" && (
              <AppBar library={library} sync={sync} onSearch={focusSearch} />
            )}
            <main className="flex-1 overflow-y-auto px-4 pt-1 pb-24">{content}</main>
            <BottomNav section={section} />
            <NavDrawer
              open={overlay?.name === "drawer"}
              count={counts.catalog}
              sync={sync}
            />
          </div>
        )}

        <LibraryPicker open={overlay?.name === "libraries"} desktop={desktop} />
        <NewLibraryDialog
          open={overlay?.name === "newLibrary"}
          desktop={desktop}
          onCreated={(name) => setNotice(t("libraries.created", { name }))}
        />
        <DeleteLibraryDialog
          open={overlay?.name === "deleteLibrary"}
          desktop={desktop}
          onDone={(deleted, active) =>
            setNotice(t("libraries.deleted", { name: deleted, active: nameOfId(active) }))
          }
        />
        <LeaveLibraryDialog
          open={overlay?.name === "leaveLibrary"}
          desktop={desktop}
          onDone={(left, active) =>
            setNotice(t("libraries.left", { name: left, active: nameOfId(active) }))
          }
        />
        {overlay?.name === "move" && (
          <MoveBookDialog
            open
            desktop={desktop}
            editionId={overlay.editionId}
            title={overlay.title}
            onMoved={(to) =>
              setNotice(t("libraries.moved", { title: overlay.title, name: nameOfId(to) }))
            }
          />
        )}
        <FilterPanel
          open={overlay?.name === "filters"}
          desktop={desktop}
          filter={filter}
          onChange={setFilter}
        />
        {overlay?.name === "add" && (
          <AddBookDialog
            open
            desktop={desktop}
            library={library}
            prefill={overlay.prefill}
            onOpenChange={(open) => !open && closeOverlay()}
            onCreated={(result, title) => void handleCreated(result, title, overlay.wishId)}
          />
        )}
        <SettingsDialog
          open={overlay?.name === "settings"}
          onOpenChange={(open) => !open && closeOverlay()}
        />
        <BackupsDialog
          open={overlay?.name === "backups"}
          onOpenChange={(open) => !open && closeOverlay()}
          onChanged={reload}
        />
        <BurstDialog
          open={overlay?.name === "burst"}
          onOpenChange={(open) => !open && closeOverlay()}
          onChanged={reload}
        />
        <PairDialog
          open={overlay?.name === "pair"}
          desktop={desktop}
          onPaired={(message) => {
            setNotice(message);
            reload();
          }}
        />
        {!desktop && (
          <BookDetailView
            editionId={bookId}
            desktop={false}
            onClose={closeOverlay}
            onChanged={reload}
          />
        )}
      </CatalogOptionsContext.Provider>
    </LibrariesContext.Provider>
  );
}
