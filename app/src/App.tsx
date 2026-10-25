import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { CircleCheck } from "lucide-react";

import { AccountDialog } from "@/components/account/account-dialog";
import { useArrivedLink } from "@/lib/arrived-link";
import { InvitationDialog } from "@/components/cloud/invitation-dialog";
import { OtherOwnerDialog } from "@/components/cloud/other-owner-dialog";
import { DownloadWait } from "@/components/cloud/download-wait";
import { AccountNotice } from "@/components/account/account-notice";
import { AuthDialog } from "@/components/account/auth-dialog";
import { DevicesDialog as AccountDevicesDialog } from "@/components/account/devices-dialog";
import { TwoFactorDialog } from "@/components/account/two-factor-dialog";
import { CloudPickDialog } from "@/components/cloud/cloud-pick-dialog";
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
  LeaveForGoodDialog,
  LeaveLibraryDialog,
} from "@/components/libraries/remove-library-dialogs";
import { type Libraries, LibrariesContext } from "@/components/libraries-context";
import { PairDialog } from "@/components/pair-dialog";
import { AboutDialog } from "@/components/about-dialog";
import { ProvidersDialog } from "@/components/providers-dialog";
import { ServerServicesDialog } from "@/components/server-services-dialog";
import { WELCOME_VERSION, WelcomeDialog } from "@/components/welcome-dialog";
import { WhatsNewDialog } from "@/components/whats-new-dialog";
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
import { Button } from "@/components/ui/button";
import {
  type BookFilter,
  type CatalogOptions,
  type CreatedBook,
  type Holding,
  type InvitationAcceptedNotice,
  type LibrarySummary,
  addToSeries,
  catalogOptions,
  cloudEventsWake,
  getBook,
  lanStatus,
  listBooks,
  listLibraries,
  pendingInvitation,
  personName,
  setActiveLibrary,
  setWelcomeSeen,
  setWhatsNewSeen,
  welcomeSeen,
  whatsNewPending,
  type WhatsNew,
} from "@/lib/api";
import { refreshAccount, useAccount } from "@/lib/account";
import { cloudNotices, refreshCloudStatus, runCloudSync, watchCloudConnection } from "@/lib/cloud";
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
import type { CatalogView } from "@/components/catalog-views";
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

/** How often the LAN is asked while the app is in the foreground. */
const LAN_POLL_MS = 5 * 60_000;

/**
 * Before the welcome has the person's name there is no library (D97): nothing but the welcome is mounted, so no
 * screen or hook asks for data that is not there. Once the name is in, the app starts as usual.
 */
export default function App() {
  const [gate, setGate] = useState<"loading" | "naming" | "ready">("loading");
  const [restSeen, setRestSeen] = useState(false);
  const [news, setNews] = useState<WhatsNew | null>(null);
  useEffect(() => {
    // The news are asked here, once, before the welcome has a say: a device that never saw the welcome is new and
    // gets none. They show after the welcome, never together with it.
    Promise.all([personName(), welcomeSeen(), whatsNewPending().catch(() => null)])
      .then(([name, seen, pending]) => {
        setNews(pending);
        setRestSeen(seen >= WELCOME_VERSION);
        setGate(name === null ? "naming" : "ready");
      })
      // If it cannot be asked, start as usual: at worst the first screen shows its own error.
      .catch(() => setGate("ready"));
  }, []);

  if (gate === "loading") return <div className="min-h-screen bg-background" />;
  if (gate === "naming") {
    return (
      <WelcomeDialog
        open
        onlyName={restSeen}
        onDone={() => {
          if (!restSeen) void setWelcomeSeen(WELCOME_VERSION);
          setGate("ready");
        }}
      />
    );
  }
  return <AppShell news={news} />;
}

function AppShell({ news }: { news: WhatsNew | null }) {
  const { t } = useTranslation();
  const { section, overlay } = useLocation();
  const desktop = useDesktop();
  const [state, setState] = useState<LoadState>({ status: "loading" });
  const [missingCount, setMissingCount] = useState(0);
  const [options, setOptions] = useState<CatalogOptions>(emptyOptions);
  const [filter, setFilter] = useState<BookFilter>({});
  const [notice, setNotice] = useState<string | null>(null);
  // What pressing a notice does, tied to its text: any other notice replaces it and the action goes with it.
  const [noticeAction, setNoticeAction] = useState<{ text: string; run: () => void } | null>(null);
  // `null` until it is known whether the welcome shows: the news wait for that.
  const [welcome, setWelcome] = useState<boolean | null>(null);
  const [newsOpen, setNewsOpen] = useState(
    news !== null && (news.switches.length > 0 || news.versions.length > 0),
  );
  useEffect(() => {
    void welcomeSeen()
      .then((seen) => setWelcome(seen < WELCOME_VERSION))
      .catch(() => setWelcome(false));
  }, []);
  const closeNews = () => {
    setNewsOpen(false);
    void setWhatsNewSeen();
  };
  const closeWelcome = () => {
    setWelcome(false);
    void setWelcomeSeen(WELCOME_VERSION);
  };
  const [version, setVersion] = useState(0);
  const [searching, setSearching] = useState(false);
  const [everywhere, setEverywhere] = useState(false);
  const [allLibraries, setAllLibraries] = useState<LibrarySummary[]>([]);
  const account = useAccount();
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

  // A link opened before signing in waits in Rust: once the account is in and nothing else is open, the
  // person comes back to it, once per sign-in.
  const resumeInvitation = useRef(false);
  const wasSignedIn = useRef(false);
  useEffect(() => {
    if (account.signedIn && !wasSignedIn.current) resumeInvitation.current = true;
    wasSignedIn.current = account.signedIn;
    if (!account.signedIn || !resumeInvitation.current || overlay !== null) return;
    resumeInvitation.current = false;
    void pendingInvitation()
      .then((waiting) => waiting.state !== "nothing" && openOverlay({ name: "invitation" }))
      .catch(() => {});
  }, [account.signedIn, overlay]);

  // A link the system handed to the app: it opens «I have an invitation» when nothing else is in front.
  useArrivedLink(overlay !== null || welcome === true || newsOpen, overlay?.name === "auth");

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

  // The cloud link of each library, local and instant: rows and the sync indicator read it.
  useEffect(() => {
    void refreshCloudStatus();
  }, [account.signedIn, version]);

  // Syncs the cloud and tells what is worth telling: a library that came down, one that was
  // not mixed with another, one the cloud lost (D84). A failure only updates the indicator.
  const syncTheCloud = useCallback(
    async (minAgeMs: number): Promise<string[]> => {
      const reports = await runCloudSync(minAgeMs);
      if (!reports || reports.length === 0) return [];
      const libraries = await listLibraries().catch(() => []);
      return cloudNotices(reports, libraries, t, true);
    },
    [t],
  );

  // Signing in on this device pulls the account's libraries now, not at the next minute.
  useEffect(() => {
    if (!account.signedIn) return;
    void syncTheCloud(60_000).then((messages) => {
      if (messages.length > 0) setNotice(messages.join(" "));
      reload();
    });
  }, [account.signedIn, syncTheCloud, reload]);

  // The live connection to the server's hub (D75): the indicator follows it.
  useEffect(() => watchCloudConnection(), []);

  // Quietly catch up with every reachable paired device and the cloud when the app opens,
  // and again when it comes back to the foreground (a phone arriving home), at most once a
  // minute. The LAN goes first and a cloud failure never gets in its way. The same pass
  // runs at once, with no minute limit, after this device's own changes (`local-changed`)
  // and after a paired device pushes something new to us (`library-changed`), so this
  // device passes it on to the others and to the cloud. The cloud telling us something
  // changed (`cloud-changed`) syncs only the cloud. While the app is in the foreground, the
  // LAN is also asked every five minutes, since a LAN server cannot reach its clients (D88);
  // the cloud joins that pass only when the LAN brought something new.
  useEffect(() => {
    let last = 0;
    let running = false;
    const again = {
      marked: false,
      /** Reads the mark and clears it. */
      take() {
        const was = this.marked;
        this.marked = false;
        return was;
      },
    };
    const pass = async (lanOnly: boolean) => {
      let removed = 0;
      let received = 0;
      let messages: string[] = [];
      const status = await lanStatus().catch(() => null);
      for (const device of status?.servers ?? []) {
        const report = await syncDevice(device.deviceId).catch(() => null);
        removed += report?.removed.length ?? 0;
        received += report?.received.received ?? 0;
      }
      if (!lanOnly || received > 0) messages = await syncTheCloud(0);
      if (removed > 0) messages = [t("sharing.removed", { count: removed }), ...messages];
      if (messages.length > 0) setNotice(messages.join(" "));
      // Reload even without changes: the sync indicator shows the new time.
      reload();
    };
    // `force` skips the minute limit. A forced call while a pass runs is not dropped: it
    // makes the pass run once more when it ends.
    const syncAll = async (force: boolean) => {
      if (running) {
        if (force) again.marked = true;
        return;
      }
      if (!force && Date.now() - last < 60_000) return;
      running = true;
      try {
        again.take();
        do {
          last = Date.now();
          await pass(false);
        } while (again.take());
      } finally {
        running = false;
      }
    };
    const onCloudChanged = async () => {
      const messages = await syncTheCloud(0);
      if (messages.length > 0) setNotice(messages.join(" "));
      reload();
    };
    // Skipped while another pass runs: that one already asks the LAN.
    const askTheLan = async () => {
      if (running || document.visibilityState !== "visible") return;
      running = true;
      try {
        await pass(true);
      } finally {
        running = false;
      }
      if (again.take()) void syncAll(true);
    };
    const timer = window.setInterval(() => void askTheLan(), LAN_POLL_MS);
    const onVisible = () => {
      if (document.visibilityState !== "visible") return;
      // Android may have killed the live socket while the app slept.
      void cloudEventsWake().catch(() => {});
      void syncAll(false);
    };
    void syncAll(false);
    document.addEventListener("visibilitychange", onVisible);
    const unlisten = [
      listen("cloud-changed", () => void onCloudChanged()),
      // Somebody accepted an invitation of this account (D43). The event carries no personal data: the app
      // syncs (the `cloud-changed` that comes with it) and the screen of People says who.
      listen<InvitationAcceptedNotice>("invitation-accepted", (event) => {
        const text = t("invitations.notice.accepted", { library: event.payload.libraryName });
        setNotice(text);
        setNoticeAction({
          text,
          run: () => {
            void setActiveLibrary(event.payload.library)
              .then(() => {
                reload();
                navigate("people");
              })
              .catch(() => {});
          },
        });
      }),
      listen("local-changed", () => void syncAll(true)),
      listen("library-changed", () => {
        reload();
        void syncAll(true);
      }),
    ];
    return () => {
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisible);
      unlisten.forEach((stop) => void stop.then((fn) => fn()));
    };
  }, [reload, syncTheCloud, t]);

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

  /** The desktop segmented control («Lo tengo · Me falta · Todo · Sagas y colecciones», D61). */
  const onViewChange = (next: CatalogView) => {
    if (next === "missing" || next === "series") {
      navigate(next);
      return;
    }
    navigate("catalog");
    setFilter((f) => ({ ...f, holding: next === "all" ? undefined : next }));
  };

  const page =
    section === "check" ? (
      <CheckPage
        desktop={desktop}
        version={version}
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
      <SeriesPage desktop={desktop} version={version} onViewChange={onViewChange} />
    ) : section === "help" ? (
      <HelpPage desktop={desktop} onShowWelcome={() => setWelcome(true)} />
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
          <AlertDescription className="flex flex-wrap items-center gap-x-3 gap-y-1">
            {notice}
            {noticeAction?.text === notice && (
              <Button variant="link" size="sm" className="h-auto p-0" onClick={noticeAction.run}>
                {t("invitations.notice.see")}
              </Button>
            )}
          </AlertDescription>
        </Alert>
      )}
      <DownloadWait />
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
                    addCopy={overlay?.name === "book" && overlay.addCopy === true}
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
          <LeaveForGoodDialog
            open={overlay?.name === "leaveForGood"}
            desktop={desktop}
            onDone={(left, removed) =>
              setNotice(t(removed ? "libraries.exitDone" : "libraries.exitPending", { name: left }))
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
              onChanged={reload}
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
          <WelcomeDialog open={welcome === true} onDone={closeWelcome} />
          {news && (
            <WhatsNewDialog open={newsOpen && welcome === false} news={news} onClose={closeNews} />
          )}
          <ProvidersDialog
            open={overlay?.name === "providers"}
            onOpenChange={(open) => !open && closeOverlay()}
          />
          <AboutDialog
            open={overlay?.name === "about"}
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
            googleError={overlay?.name === "auth" ? overlay.googleError : undefined}
          />
          <AccountDialog open={overlay?.name === "account"} desktop={desktop} />
          <AccountDevicesDialog open={overlay?.name === "accountDevices"} desktop={desktop} />
          <CloudPickDialog
            open={overlay?.name === "cloudPick"}
            onDone={(count) => setNotice(t("cloud.pick.done", { count }))}
          />
          <InvitationDialog
            open={overlay?.name === "invitation"}
            errorKey={overlay?.name === "invitation" ? overlay.errorKey : undefined}
            desktop={desktop}
            onDone={setNotice}
          />
          <TwoFactorDialog
            open={overlay?.name === "twoFactor"}
            desktop={desktop}
            codes={overlay?.name === "twoFactor" ? overlay.codes : undefined}
          />
          <AccountNotice libraries={allLibraries} />
          <ServerServicesDialog
            blocked={welcome !== false || (newsOpen && news !== null) || overlay?.name === "auth"}
          />
          <OtherOwnerDialog desktop={desktop} />
          {!desktop && (
            <BookDetailView
              editionId={bookId}
              addCopy={overlay?.name === "book" && overlay.addCopy === true}
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
