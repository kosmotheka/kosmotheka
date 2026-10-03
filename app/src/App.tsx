import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { CircleCheck } from "lucide-react";

import { AccountDialog } from "@/components/account/account-dialog";
import { AccountNotice } from "@/components/account/account-notice";
import { AuthDialog } from "@/components/account/auth-dialog";
import { DevicesDialog as AccountDevicesDialog } from "@/components/account/devices-dialog";
import { TwoFactorDialog } from "@/components/account/two-factor-dialog";
import { AddBookDialog } from "@/components/add-book-dialog";
import { AddSearch } from "@/components/add-search";
import { AppLockScreen } from "@/components/app-lock-screen";
import { ScannerOverlay } from "@/components/scanner-overlay";
import { BackupsDialog } from "@/components/backups-dialog";
import { BookDetailView } from "@/components/book-detail";
import { BurstDialog } from "@/components/burst-dialog";
import { FilterPanel, isFiltering } from "@/components/catalog-filters";
import { CatalogOptionsContext, emptyOptions } from "@/components/catalog-options";
import { MoveBookDialog } from "@/components/libraries/move-book-dialog";
import { NewLibraryDialog } from "@/components/libraries/new-library-dialog";
import {
  DeleteLibraryDialog,
  DuplicateLibraryDialog,
  LeaveLibraryDialog,
} from "@/components/libraries/remove-library-dialogs";
import { type Libraries, LibrariesContext } from "@/components/libraries-context";
import { PairDialog } from "@/components/pair-dialog";
import { ProvidersDialog } from "@/components/providers-dialog";
import { NewSeriesDialog } from "@/components/new-series-dialog";
import { RequestNotices } from "@/components/request-notices";
import { RequestsContext, useRequestsState } from "@/components/requests-context";
import { SettingsDialog } from "@/components/settings-dialog";
import { SeriesDetailView } from "@/components/series-detail";
import { SeriesPickerDialog } from "@/components/series-picker-dialog";
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
  type Holding,
  type LibrarySummary,
  addToSeries,
  catalogOptions,
  getBook,
  lanStatus,
  listBooks,
  listLibraries,
  setActiveLibrary,
} from "@/lib/api";
import { refreshAccount } from "@/lib/account";
import { closeOverlay, navigate, openOverlay, useLocation } from "@/lib/navigation";
import { syncDevice } from "@/lib/sync-errors";
import { type LoadState, CatalogPage } from "@/pages/catalog-page";
import { CheckPage } from "@/pages/check-page";
import { DevicesPage } from "@/pages/devices-page";
import { HelpPage } from "@/pages/help-page";
import { LibraryPage } from "@/pages/library-page";
import { PeoplePage } from "@/pages/people-page";
import { RequestsPage } from "@/pages/requests-page";
import { SeriesPage } from "@/pages/series-page";
import { TagsPage } from "@/pages/tags-page";

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
  const [missingCount, setMissingCount] = useState(0);
  const [options, setOptions] = useState<CatalogOptions>(emptyOptions);
  const [filter, setFilter] = useState<BookFilter>({});
  const [notice, setNotice] = useState<string | null>(null);
  const [version, setVersion] = useState(0);
  const [searching, setSearching] = useState(false);
  const [everywhere, setEverywhere] = useState(false);
  const [allLibraries, setAllLibraries] = useState<LibrarySummary[]>([]);
  const sync = useSyncStatus(version);
  const requests = useRequestsState(version);

  const activeLibrary = allLibraries.find((l) => l.active) ?? null;
  const library = activeLibrary?.name || t("nav.myLibrary");
  const nameOfId = (id: string) =>
    allLibraries.find((l) => l.id === id)?.name || t("nav.myLibrary");

  const activeFilter = useDebounced(filter, 200);
  // "In every library" only means something while searching.
  const searchEverywhere = everywhere && Boolean(activeFilter.query?.trim());
  // The desktop segmented control and the "Missing" nav entry both drive this,
  // the nav entry by forcing it regardless of what the control last showed (D61).
  // Not debounced, unlike the rest of the filter: picking a view should react at once.
  const holding: Holding | undefined = section === "missing" ? "missing" : filter.holding;

  useEffect(() => {
    let cancelled = false;
    listLibraries()
      .then((loaded) => !cancelled && setAllLibraries(loaded))
      .catch(() => {});
    listBooks({ ...activeFilter, holding }, searchEverywhere)
      .then((books) => !cancelled && setState({ status: "ready", books }))
      .catch(
        (error: unknown) => !cancelled && setState({ status: "error", message: String(error) }),
      );
    catalogOptions()
      .then((loaded) => !cancelled && setOptions(loaded))
      .catch(() => {});
    // For the sidebar/bottom-nav badge, regardless of the current view's own filters.
    listBooks({ holding: "missing" })
      .then((loaded) => !cancelled && setMissingCount(loaded.length))
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [activeFilter, searchEverywhere, version, holding]);

  /** Reloads books, the missing list and suggestions after any change. */
  const reload = useCallback(() => setVersion((v) => v + 1), []);

  useEffect(() => {
    void refreshAccount();
  }, []);

  // The device was reset (D81): its identity is new, so the welcome flag from
  // before no longer applies and a stale `?mock`-style view would be wrong.
  useEffect(() => {
    const unlisten = listen("device-reset", () => {
      try {
        localStorage.removeItem("kosmotheka.welcomeSeen");
      } catch {
        // Nothing to do: at worst the welcome screen shows again.
      }
      window.location.reload();
    });
    return () => void unlisten.then((stop) => stop());
  }, []);

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
    missing: missingCount,
  };

  const handleCreated = (result: CreatedBook, title: string) => {
    const message = t(result.existingEdition ? "catalog.addedCopy" : "catalog.added", { title });
    setNotice(result.coverFailed ? `${message} ${t("catalog.coverFailed")}` : message);
    reload();
    // A series gap's «Apuntar aquí» (D61): the new work joins the series at that number,
    // then the series page comes back instead of the catalog.
    if (overlay?.name === "add" && overlay.intoSeries) {
      const { seriesId, position } = overlay.intoSeries;
      getBook(result.editionId)
        .then((book) => addToSeries(seriesId, book.workId, position))
        .then(() => openOverlay({ name: "series", seriesId }))
        .catch(() => {});
    }
  };

  const bookId = overlay?.name === "book" ? overlay.editionId : null;

  const switchTo = async (id: string) => {
    await setActiveLibrary(id);
    reload();
  };
  const libraries: Libraries = {
    all: allLibraries,
    active: activeLibrary,
    switchTo,
    reload,
  };

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

  /** A genre or tag chip on a work's page opens the catalog filtered by it (6.7). */
  const filterAndShow = (patch: Partial<BookFilter>) => {
    setFilter((f) => ({ ...f, ...patch }));
    navigate("catalog");
  };

  /** The desktop segmented control («Lo tengo · Me falta · Todo», D61). */
  const onViewChange = (next: Holding | undefined) => {
    if (next === "missing") {
      navigate("missing");
      return;
    }
    navigate("catalog");
    setFilter((f) => ({ ...f, holding: next }));
  };

  const page =
    section === "check" ? (
      <CheckPage
        desktop={desktop}
        onChanged={reload}
        onOpenBook={(libraryId, editionId) => void openBookIn(libraryId, editionId)}
      />
    ) : section === "library" ? (
      <LibraryPage desktop={desktop} />
    ) : section === "tags" ? (
      <TagsPage desktop={desktop} version={version} onChanged={reload} />
    ) : section === "devices" ? (
      <DevicesPage desktop={desktop} version={version} onChanged={reload} onNotice={setNotice} />
    ) : section === "people" ? (
      <PeoplePage desktop={desktop} version={version} onNotice={setNotice} />
    ) : section === "requests" ? (
      <RequestsPage desktop={desktop} version={version} onChanged={reload} onNotice={setNotice} />
    ) : section === "series" ? (
      <SeriesPage desktop={desktop} version={version} />
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
        onOpenBook={(libraryId, editionId) => void openBookIn(libraryId, editionId)}
        selectedId={desktop ? bookId : null}
        holding={holding}
        onViewChange={onViewChange}
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
      <RequestsContext.Provider value={requests}>
        <CatalogOptionsContext.Provider value={options}>
          <AppLockScreen />
          <ScannerOverlay />
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
                  missing={section === "missing"}
                />
                <div className="flex min-h-0 flex-1">
                  <main className="min-w-0 flex-1 overflow-y-auto px-6 pt-5 pb-8">{content}</main>
                  <BookDetailView
                    editionId={bookId}
                    desktop
                    onClose={closeOverlay}
                    onChanged={reload}
                    onNotice={setNotice}
                    onFilter={filterAndShow}
                  />
                </div>
              </div>
            </div>
          ) : (
            <div className="flex h-dvh flex-col pt-[env(safe-area-inset-top)]">
              {!["check", "library", "tags", "devices", "people", "requests", "series"].includes(
                section,
              ) && <AppBar library={library} sync={sync} onSearch={focusSearch} />}
              <main className="flex-1 overflow-y-auto px-4 pt-1 pb-24">{content}</main>
              <BottomNav section={section} />
              <NavDrawer open={overlay?.name === "drawer"} count={counts.catalog} sync={sync} />
            </div>
          )}

          <RequestNotices version={version} desktop={desktop} />
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
              setNotice(
                t("libraries.deleted", {
                  name: deleted,
                  active: nameOfId(active),
                }),
              )
            }
          />
          <DuplicateLibraryDialog
            open={overlay?.name === "duplicateLibrary"}
            desktop={desktop}
            onDone={(name) => setNotice(t("libraries.duplicated", { name }))}
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
                setNotice(
                  t("libraries.moved", {
                    title: overlay.title,
                    name: nameOfId(to),
                  }),
                )
              }
            />
          )}
          <FilterPanel
            open={overlay?.name === "filters"}
            desktop={desktop}
            filter={filter}
            onChange={setFilter}
          />
          {overlay?.name === "add" && overlay.prefill && (
            // A title already typed in the catalog search (no results found): straight
            // to the form, not another search step (D60).
            <AddBookDialog
              open
              desktop={desktop}
              library={library}
              prefill={overlay.prefill}
              missing={overlay.missing}
              onOpenChange={(open) => !open && closeOverlay()}
              onCreated={handleCreated}
              onRequested={(requested) => {
                setNotice(t(requested ? "ask.sent" : "ask.done"));
                reload();
              }}
            />
          )}
          {overlay?.name === "add" && !overlay.prefill && (
            <AddSearch
              open
              desktop={desktop}
              library={library}
              missing={overlay.missing}
              onOpenChange={(open) => !open && closeOverlay()}
              onCreated={handleCreated}
              onRequested={(requested) => {
                setNotice(t(requested ? "ask.sent" : "ask.done"));
                reload();
              }}
            />
          )}
          <SettingsDialog
            open={overlay?.name === "settings"}
            desktop={desktop}
            onOpenChange={(open) => !open && closeOverlay()}
          />
          <ProvidersDialog
            open={overlay?.name === "providers"}
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
          <SeriesDetailView
            seriesId={overlay?.name === "series" ? overlay.seriesId : null}
            desktop={desktop}
            onClose={closeOverlay}
            onChanged={reload}
          />
          <NewSeriesDialog open={overlay?.name === "newSeries"} desktop={desktop} />
          {overlay?.name === "seriesPicker" && (
            <SeriesPickerDialog
              open
              desktop={desktop}
              workId={overlay.workId}
              workTitle={overlay.workTitle}
              onClose={closeOverlay}
              onAdded={() => {
                closeOverlay();
                reload();
              }}
            />
          )}
          <PairDialog
            open={overlay?.name === "pair"}
            desktop={desktop}
            onPaired={(message) => {
              setNotice(message);
              reload();
            }}
          />
          <AuthDialog
            open={overlay?.name === "auth"}
            initialStep={overlay?.name === "auth" ? overlay.step : undefined}
          />
          <AccountDialog open={overlay?.name === "account"} desktop={desktop} />
          <AccountDevicesDialog open={overlay?.name === "accountDevices"} desktop={desktop} />
          <TwoFactorDialog
            open={overlay?.name === "twoFactor"}
            desktop={desktop}
            codes={overlay?.name === "twoFactor" ? overlay.codes : undefined}
          />
          <AccountNotice libraries={allLibraries} />
          {!desktop && (
            <BookDetailView
              editionId={bookId}
              desktop={false}
              onClose={closeOverlay}
              onChanged={reload}
              onNotice={setNotice}
              onFilter={filterAndShow}
            />
          )}
        </CatalogOptionsContext.Provider>
      </RequestsContext.Provider>
    </LibrariesContext.Provider>
  );
}
