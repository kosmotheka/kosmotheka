import { useSyncExternalStore } from "react";

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
  "catalog" | "check" | "missing" | "library" | "devices" | "people" | "requests" | "help";

export type Overlay =
  | { name: "drawer" }
  | { name: "libraries" }
  | { name: "pair" }
  | { name: "settings" }
  | { name: "providers" }
  | { name: "backups" }
  | { name: "filters" }
  | { name: "burst" }
  | { name: "add"; prefill?: { isbn?: string; title?: string; author?: string } }
  | { name: "book"; editionId: string }
  | { name: "newLibrary" }
  | { name: "deleteLibrary" }
  | { name: "leaveLibrary" }
  | { name: "move"; editionId: string; title: string };

export interface Location {
  section: Section;
  overlay: Overlay | null;
}

const SECTIONS: Record<string, Section> = {
  "": "catalog",
  check: "check",
  missing: "missing",
  library: "library",
  devices: "devices",
  people: "people",
  requests: "requests",
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
