import { useSyncExternalStore } from "react";

// Desktop layout from 1024 px (docs/design/brief.md): sidebar instead of the
// bottom bar. A tablet in landscape gets it too.
const query = window.matchMedia("(min-width: 1024px)");

export function useDesktop(): boolean {
  return useSyncExternalStore(
    (listener) => {
      query.addEventListener("change", listener);
      return () => query.removeEventListener("change", listener);
    },
    () => query.matches,
  );
}
