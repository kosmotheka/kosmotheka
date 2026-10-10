import type { TFunction } from "i18next";
import { listen } from "@tauri-apps/api/event";
import { useSyncExternalStore } from "react";

import {
  type CloudConnection,
  type CloudLibraryReport,
  type CloudOutcome,
  type LibraryCloudStatus,
  type LibrarySummary,
  type Role,
  accountStatus,
  cloudConnection,
  cloudErrorKey,
  cloudStatus,
  isCloudOffline,
  syncCloud,
} from "@/lib/api";

// What the app knows about the cloud, cached between renders like lib/account.ts. Rust
// keeps the link (local, in `cloudStatus`); the rest is what this session's syncs saw.

export interface CloudState {
  /** One entry per library, from `cloudStatus` (local, no network). */
  status: LibraryCloudStatus[];
  syncing: boolean;
  /** The last sync could not reach the server. */
  offline: boolean;
  /** Translation key of the last failure that was not «offline». */
  errorKey: string | null;
  /** When each library last synced with the cloud in this session. */
  syncedAt: Readonly<Record<string, number>>;
  /** What the last sync did to each library. */
  outcomes: Readonly<Record<string, CloudOutcome>>;
  /** Connected to the server's hub: its changes arrive at once (D75). */
  live: boolean;
  /** The server has a hub and this device is signed in: «live» means something. */
  available: boolean;
}

const initial: CloudState = {
  status: [],
  syncing: false,
  offline: false,
  errorKey: null,
  syncedAt: {},
  outcomes: {},
  live: false,
  available: false,
};

let state = initial;
const listeners = new Set<() => void>();

function update(patch: Partial<CloudState>) {
  state = { ...state, ...patch };
  listeners.forEach((listener) => listener());
}

export function useCloud(): CloudState {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => state,
  );
}

/** Reads the cloud link of every library (local, instant). */
export async function refreshCloudStatus(): Promise<LibraryCloudStatus[]> {
  const status = await cloudStatus().catch(() => []);
  update({ status });
  return status;
}

/** Records a library that just went up (or synced) so its row shows «up to date». */
export function recordSynced(report: CloudLibraryReport) {
  update({
    syncedAt: { ...state.syncedAt, [report.library]: Date.now() },
    outcomes: { ...state.outcomes, [report.library]: report.outcome },
    offline: false,
    errorKey: null,
  });
}

let lastRunAt = 0;
// A sync asked for while another one runs is not lost: it runs once more when that ends.
const pending = {
  marked: false,
  /** Reads the mark and clears it. */
  take() {
    const was = this.marked;
    this.marked = false;
    return was;
  },
};

// Read through a function so the checks after an `await` are not narrowed away.
const isSyncing = () => state.syncing;

/** Follows the live connection: the first state, then the core's `cloud-connection` events. */
export function watchCloudConnection(): () => void {
  let stopped = false;
  void cloudConnection()
    .then((connection) => !stopped && update(connection))
    .catch(() => {});
  const unlisten = listen<CloudConnection>("cloud-connection", (event) => update(event.payload));
  return () => {
    stopped = true;
    void unlisten.then((stop) => stop());
  };
}

/**
 * Syncs every cloud library of the account. Never throws: a cloud failure only updates
 * the indicator, since the app works without it. Returns what each library did, or `null`
 * when it did not run (signed out, run less than `minAgeMs` ago, or failed) or when another
 * sync was running: a call with `minAgeMs` 0 then marks it to run once more, and the one
 * running returns the reports of both.
 */
export async function runCloudSync(minAgeMs = 0): Promise<CloudLibraryReport[] | null> {
  if (isSyncing()) {
    if (minAgeMs === 0) pending.marked = true;
    return null;
  }
  if (Date.now() - lastRunAt < minAgeMs) return null;
  const account = await accountStatus().catch(() => null);
  if (!account?.signedIn) {
    update({
      ...initial,
      status: state.status.map((s) => ({ ...s, signedIn: false })),
      live: state.live,
      available: state.available,
    });
    return null;
  }
  // Another call may have started while the account was being read.
  if (isSyncing()) {
    if (minAgeMs === 0) pending.marked = true;
    return null;
  }
  lastRunAt = Date.now();
  update({ syncing: true });
  const all: CloudLibraryReport[] = [];
  try {
    pending.take();
    do {
      const reports = await syncCloud();
      all.push(...reports);
      const now = Date.now();
      const syncedAt = { ...state.syncedAt };
      const outcomes: Record<string, CloudOutcome> = {};
      for (const report of reports) {
        outcomes[report.library] = report.outcome;
        if (report.outcome === "synced") syncedAt[report.library] = now;
      }
      update({ offline: false, errorKey: null, syncedAt, outcomes });
    } while (pending.take());
    return all;
  } catch (error) {
    pending.take();
    if (isCloudOffline(error)) update({ offline: true, errorKey: null });
    else update({ offline: false, errorKey: cloudErrorKey(error) });
    return null;
  } finally {
    update({ syncing: false });
    await refreshCloudStatus();
  }
}

// Background syncs are frequent: each outcome is worth telling once per session.
const told = new Set<string>();

/**
 * The messages worth showing for a sync's outcomes. In the background (`once`) the same
 * outcome of the same library is told a single time; after «Sync now» it always is.
 */
export function cloudNotices(
  reports: CloudLibraryReport[],
  libraries: LibrarySummary[],
  t: TFunction,
  once: boolean,
): string[] {
  const messages: string[] = [];
  // Rust says it only the first time (D76): no need to remember it here.
  const nameOf = (id: string, given?: string) =>
    given || libraries.find((l) => l.id === id)?.name || t("nav.myLibrary");
  const joined = reports.flatMap(({ library, joined: who }) =>
    who
      ? [
          {
            name: nameOf(library),
            as: [who.person, who.role && t(`people.roles.${who.role}`).toLocaleLowerCase()]
              .filter(Boolean)
              .join(" · "),
          },
        ]
      : [],
  );
  if (joined.length === 1) {
    messages.push(t("cloud.notice.joinedOne", joined[0]));
  } else if (joined.length > 1) {
    messages.push(
      t("cloud.notice.joinedMany", {
        count: joined.length,
        list: joined.map((j) => `«${j.name}», ${j.as}`).join("; "),
      }),
    );
  }
  const downloaded = reports.filter((report) => report.outcome === "downloaded");
  if (downloaded.length > 0) {
    // One summary for the whole session: how many came down and who this device is in each.
    const roleOf = (role: Role | null) => role && t(`people.roles.${role}`).toLocaleLowerCase();
    const lines = downloaded.flatMap(({ library, me }) =>
      me
        ? [
            t("cloud.notice.downloadedAs", {
              name: nameOf(library),
              as: [me.person, roleOf(me.role)].filter(Boolean).join(" · "),
            }),
          ]
        : [],
    );
    messages.push(
      [
        t("cloud.notice.downloadedCount", { count: downloaded.length }),
        ...lines,
        t("cloud.notice.downloadedNotUploaded"),
      ].join(" "),
    );
  }
  for (const report of reports) {
    if (report.outcome === "downloaded") continue;
    const key = `${report.outcome}:${report.library}`;
    if (once && told.has(key)) continue;
    const name = nameOf(report.library, report.libraryName);
    const message = (
      {
        library_conflict: () => t("cloud.notice.libraryConflict", { name }),
        missing_in_cloud: () => t("cloud.notice.missingInCloud", { name }),
        deleted_here: () => t("cloud.notice.deletedHere"),
        removed: () => t("cloud.notice.removed", { name }),
        access_withdrawn: () => t("cloud.notice.accessWithdrawn", { name }),
        not_linked: () => t("cloud.notice.notLinked"),
        failed: () => t("cloud.notice.failed", { name }),
      } as Partial<Record<CloudOutcome, () => string>>
    )[report.outcome];
    if (!message) continue;
    told.add(key);
    messages.push(message());
  }
  return messages;
}
