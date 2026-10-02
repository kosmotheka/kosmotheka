import { useSyncExternalStore } from "react";

import { type AccountStatus, accountStatus } from "@/lib/api";

// Local account state, cached between renders like lib/sync-errors.ts; the Rust
// side keeps the source of truth and `refreshAccount` just re-reads it.

const empty: AccountStatus = {
  server: "",
  official: true,
  signedIn: false,
  user: null,
  emailHint: null,
  notice: null,
};

let status: AccountStatus = empty;
const listeners = new Set<() => void>();

function set(next: AccountStatus) {
  status = next;
  listeners.forEach((listener) => listener());
}

/** Reads the account state from Rust (local, instant) and notifies subscribers. */
export async function refreshAccount(): Promise<AccountStatus> {
  const next = await accountStatus();
  set(next);
  return next;
}

export function useAccount(): AccountStatus {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => status,
  );
}
