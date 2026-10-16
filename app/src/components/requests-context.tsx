import { createContext, useContext, useEffect, useState } from "react";

import { type Role, listPeople, listRequests } from "@/lib/api";

/** What the menus need to know about requests in the active library (D73). */
export interface RequestsState {
  myRole: Role | null;
  /** Administrators: pending requests from others. Everyone else: their own pending ones. */
  pending: number;
}

const empty: RequestsState = { myRole: null, pending: 0 };

export const RequestsContext = createContext<RequestsState>(empty);

export function useRequests() {
  return useContext(RequestsContext);
}

/** Reloads whenever `version` changes: after writing, syncing or switching library. */
export function useRequestsState(version: number): RequestsState {
  const [state, setState] = useState<RequestsState>(empty);
  useEffect(() => {
    let cancelled = false;
    Promise.all([listPeople(), listRequests()])
      .then(([people, requests]) => {
        if (cancelled) return;
        const admin = people.myRole === "admin";
        const pending = requests.filter(
          (r) => r.status === "pending" && (admin ? !r.mine : r.mine),
        ).length;
        setState({ myRole: people.myRole, pending });
      })
      .catch(() => !cancelled && setState(empty));
    return () => {
      cancelled = true;
    };
  }, [version]);
  return state;
}
