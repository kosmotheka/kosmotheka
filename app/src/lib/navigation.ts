import { useEffect, useRef, useSyncExternalStore } from "react";

import type { AddBookPrefill } from "@/components/add-book-dialog";

// Navigation lives in the browser history, so the Android back button (which
// Tauri maps to "go back in the webview") and the desktop back gestures behave
// as users expect: back closes the open menu, sheet or dialog first, then
// returns to the previous section.
//
// - The section is the URL hash: `#/`, `#/check`, `#/missing`, `#/library`, `#/devices`, `#/people`, `#/requests`,
//   `#/help`.
// - An overlay (drawer, sheet, dialog) is a history entry on top of its section,
//   with its name and data in `history.state`. Opening pushes an entry; closing
//   goes back one; navigating from inside an overlay replaces the entry, so the
//   overlay closes without leaving a step behind.

export type Section =
  | "catalog"
  | "check"
  | "missing"
  | "library"
  | "tags"
  | "devices"
  | "people"
  | "requests"
  | "series"
  | "help";

export type Overlay =
  | { name: "drawer" }
  | { name: "libraries" }
  | { name: "pair" }
  | { name: "settings" }
  | { name: "providers" }
  | { name: "about" }
  | { name: "backups" }
  | { name: "filters" }
  | { name: "burst" }
  | {
      name: "add";
      prefill?: AddBookPrefill;
      /** Opened from «Me falta»: the form saves with `addMissing`/`requestMissing`, no copy (D61). */
      missing?: boolean;
      /** Opened from a series gap's «Apuntar aquí»: the new work joins the series at this
       * position once created (D61). */
      intoSeries?: { seriesId: string; position: number };
    }
  /** `addCopy`: from a saga's «+ Lo tengo», the page opens with the new copy's form. */
  | { name: "book"; editionId: string; addCopy?: boolean }
  | { name: "newLibrary" }
  | { name: "deleteLibrary" }
  | { name: "leaveLibrary" }
  | { name: "leaveForGood" }
  | { name: "duplicateLibrary" }
  | { name: "move"; editionId: string; title: string }
  | { name: "series"; seriesId: string }
  | { name: "newSeries" }
  | { name: "seriesPicker"; workId: string; workTitle: string }
  // `googleTotp` / `googleError`: the way back from Google asked for the second factor, or failed (D77).
  | { name: "auth"; step?: "signin" | "signup" | "googleTotp"; googleError?: string }
  | { name: "account" }
  | { name: "accountDevices" }
  | { name: "cloudPick" }
  // `errorKey`: the link that brought the person here was bad; the text it shows is already in the locales.
  | { name: "invitation"; errorKey?: string }
  // `codes` only when opened to show a fresh set of backup codes (regenerated from the
  // account screen) instead of starting a new setup.
  | { name: "twoFactor"; codes?: string[] };

export interface Location {
  section: Section;
  overlay: Overlay | null;
}

const SECTIONS: Record<string, Section> = {
  "": "catalog",
  check: "check",
  missing: "missing",
  library: "library",
  tags: "tags",
  devices: "devices",
  people: "people",
  requests: "requests",
  series: "series",
  help: "help",
};
const CHANGE = "kosmotheka:navigate";

function read(): Location {
  const path = window.location.hash.replace(/^#\/?/, "");
  const state = window.history.state as { overlay?: Overlay } | null;
  return { section: SECTIONS[path] ?? "catalog", overlay: state?.overlay ?? null };
}

function hashOf(section: Section) {
  return section === "catalog" ? "#/" : `#/${section}`;
}

// useSyncExternalStore needs a stable snapshot between changes.
let snapshot = read();
function refresh() {
  snapshot = read();
  window.dispatchEvent(new Event(CHANGE));
}
window.addEventListener("popstate", refresh);
window.addEventListener("hashchange", refresh);

function subscribe(listener: () => void) {
  window.addEventListener(CHANGE, listener);
  return () => window.removeEventListener(CHANGE, listener);
}

export function useLocation(): Location {
  return useSyncExternalStore(subscribe, () => snapshot);
}

/** Goes to a section. From inside an overlay, the overlay closes on the way. */
export function navigate(section: Section) {
  if (snapshot.overlay) {
    window.history.replaceState(null, "", hashOf(section));
  } else if (section !== snapshot.section) {
    window.history.pushState(null, "", hashOf(section));
  }
  refresh();
}

/** Opens an overlay; if another one is open, it takes its place. */
export function openOverlay(overlay: Overlay) {
  const url = hashOf(snapshot.section);
  if (snapshot.overlay) window.history.replaceState({ overlay }, "", url);
  else window.history.pushState({ overlay }, "", url);
  refresh();
}

/** Leaves a section the way the back button would, or goes home if there is nowhere to go back to. */
export function leaveSection() {
  if (window.history.length > 1) window.history.back();
  else navigate("catalog");
}

/** Closes the open overlay, as the back button would. */
export function closeOverlay() {
  if (snapshot.overlay) window.history.back();
}

/**
 * While `active`, the back button asks the screen first instead of leaving it: an entry of
 * its own on top of the history, like the scanner's, so back runs `onBack` with the overlay
 * still open. `onBack` returns true to stay, keeping the entry, for example while it asks
 * whether to discard changes. When `active` ends another way (saving, the screen's own
 * Cancel), the entry is dropped, so no step is left behind. Before closing the overlay or
 * opening another, await the returned `release`: otherwise that back would land on the entry.
 * Guards stack: a sheet opened over a guarded form gets back first, and closing it leaves the
 * form's guard in place.
 */
let lastGuard = 0;
const guardsIn = (state: unknown) => (state as { backGuards?: number[] } | null)?.backGuards ?? [];

export function useBackGuard(active: boolean, onBack: () => boolean): () => Promise<void> {
  const onBackRef = useRef(onBack);
  useEffect(() => {
    onBackRef.current = onBack;
  });
  const releaseRef = useRef<() => Promise<void>>(() => Promise.resolve());

  useEffect(() => {
    if (!active) return;
    const id = ++lastGuard;
    const guard = () => {
      const state: unknown = window.history.state;
      window.history.pushState({ ...(state as object), backGuards: [...guardsIn(state), id] }, "");
    };
    // Still on our entry or one above it: the scanner keeps the state of the entry below it, so
    // its own closing lands back here, as does closing a guard stacked on ours.
    const ours = () => guardsIn(window.history.state).includes(id);
    const onTop = () => guardsIn(window.history.state).at(-1) === id;
    let pushed = false;
    let done = false;
    const onPop = () => {
      if (ours()) return;
      if (onBackRef.current()) {
        guard();
        return;
      }
      done = true;
      window.removeEventListener("popstate", onPop);
    };
    const release = () => {
      window.removeEventListener("popstate", onPop);
      if (!pushed || done || !onTop()) {
        done = true;
        return Promise.resolve();
      }
      done = true;
      return new Promise<void>((resolve) => {
        window.addEventListener("popstate", () => resolve(), { once: true });
        window.history.back();
      });
    };
    // A tick later, so that a mount undone at once (React's strict mode) pushes nothing: its
    // back would land after the second mount's push and be taken for the person's.
    const timer = setTimeout(() => {
      guard();
      pushed = true;
      window.addEventListener("popstate", onPop);
    }, 0);
    releaseRef.current = release;
    return () => {
      clearTimeout(timer);
      releaseRef.current = () => Promise.resolve();
      void release();
    };
  }, [active]);

  return () => releaseRef.current();
}
