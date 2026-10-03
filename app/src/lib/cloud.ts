import type { TFunction } from "i18next";
import { useSyncExternalStore } from "react";

import {
  type CloudLibraryReport,
  type CloudOutcome,
  type LibraryCloudStatus,
  type LibrarySummary,
  accountStatus,
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
}

const initial: CloudState = {
  status: [],
  syncing: false,
  offline: false,
  errorKey: null,
  syncedAt: {},
  outcomes: {},
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

/**
 * Syncs every cloud library of the account. Never throws: a cloud failure only updates
 * the indicator, since the app works without it. Returns what each library did, or `null`
 * when it did not run (signed out, already syncing, or run less than `minAgeMs` ago) or failed.
 */
export async function runCloudSync(minAgeMs = 0): Promise<CloudLibraryReport[] | null> {
  if (state.syncing || Date.now() - lastRunAt < minAgeMs) return null;
  const account = await accountStatus().catch(() => null);
  if (!account?.signedIn) {
    update({ ...initial, status: state.status.map((s) => ({ ...s, signedIn: false })) });
    return null;
  }
  lastRunAt = Date.now();
  update({ syncing: true });
  try {
    const reports = await syncCloud();
    const now = Date.now();
    const syncedAt = { ...state.syncedAt };
    const outcomes: Record<string, CloudOutcome> = {};
    for (const report of reports) {
      outcomes[report.library] = report.outcome;
      if (report.outcome === "synced") syncedAt[report.library] = now;
    }
    update({ offline: false, errorKey: null, syncedAt, outcomes });
    return reports;
  } catch (error) {
    if (isCloudOffline(error)) update({ offline: true, errorKey: null });
    else update({ offline: false, errorKey: cloudErrorKey(error) });
    return null;
  } finally {
    update({ syncing: false });
    await refreshCloudStatus();
  }
}

// Background syncs run every minute: each outcome is worth telling once per session.
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
  for (const report of reports) {
    const key = `${report.outcome}:${report.library}`;
    if (once && report.outcome !== "downloaded" && told.has(key)) continue;
    const name = libraries.find((l) => l.id === report.library)?.name || t("nav.myLibrary");
    const message = (
      {
        downloaded: () => t("cloud.notice.downloaded", { name }),
        library_conflict: () => t("cloud.notice.libraryConflict", { name }),
        missing_in_cloud: () => t("cloud.notice.missingInCloud", { name }),
        deleted_here: () => t("cloud.notice.deletedHere"),
        not_linked: () => t("cloud.notice.notLinked"),
      } as Partial<Record<CloudOutcome, () => string>>
    )[report.outcome];
    if (!message) continue;
    told.add(key);
    messages.push(message());
  }
  return messages;
}
