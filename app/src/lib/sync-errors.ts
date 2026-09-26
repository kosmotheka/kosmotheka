import { useSyncExternalStore } from "react";

import { type SyncReport, isCommandError, syncWith } from "@/lib/api";

// Why the last sync with each device failed, while the app runs: the devices
// screen shows it on that device ("the other one has another version").

let errors: Readonly<Record<string, string>> = {};
const listeners = new Set<() => void>();

function set(deviceId: string, reason: string | null) {
  const next = { ...errors };
  if (reason === null) delete next[deviceId];
  else next[deviceId] = reason;
  errors = next;
  listeners.forEach((listener) => listener());
}

/** Syncs with a device and remembers whether it worked. */
export async function syncDevice(deviceId: string): Promise<SyncReport> {
  try {
    const report = await syncWith(deviceId);
    set(deviceId, null);
    return report;
  } catch (error) {
    set(deviceId, isCommandError(error) && error.kind === "sync" ? error.reason : "peer_error");
    throw error;
  }
}

export function useSyncErrors(): Readonly<Record<string, string>> {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => errors,
  );
}
