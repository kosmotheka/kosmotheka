// Camera barcode scanning (D35). The plugin exists only in phone builds, so it is
// loaded on demand and never touched on desktop.
//
// The plugin's full-screen camera has no close button and ignores the back
// button, so it runs "windowed": the camera is drawn behind a transparent
// WebView, and `ScannerOverlay` draws the frame and the close button on top.
// Back closes it too, through a history entry of its own.

import type { Format } from "@tauri-apps/plugin-barcode-scanner";
import { useSyncExternalStore } from "react";

export class CameraDenied extends Error {}

/** What the overlay shows while the camera is open; null when it is closed. */
export type ScanTarget = "barcode" | "qr";

let target: ScanTarget | null = null;
const listeners = new Set<() => void>();

function setTarget(next: ScanTarget | null) {
  target = next;
  document.documentElement.classList.toggle("scanning", next !== null);
  listeners.forEach((listener) => listener());
}

export function useScanTarget(): ScanTarget | null {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => target,
  );
}

// Settles the pending scan from our side. The Android plugin's `cancel` clears
// its saved call before rejecting it, so the plugin's `scan` never settles.
let settlePending: (() => void) | null = null;

/** Closes the camera; the pending scan resolves to null. */
export async function cancelScan() {
  const scanner = await import("@tauri-apps/plugin-barcode-scanner");
  try {
    await scanner.cancel();
  } finally {
    settlePending?.();
  }
}

async function scan(
  formats: (scanner: typeof import("@tauri-apps/plugin-barcode-scanner")) => Format[],
  shown: ScanTarget,
): Promise<string | null> {
  const scanner = await import("@tauri-apps/plugin-barcode-scanner");
  let permission = await scanner.checkPermissions();
  if (permission !== "granted") permission = await scanner.requestPermissions();
  if (permission !== "granted") throw new CameraDenied();

  // Same state as the entry below, so the open dialog stays open under the camera.
  window.history.pushState({ ...window.history.state, scanner: true }, "");
  // An object, not a `let`: TypeScript cannot see the listener change a local.
  const back = { popped: false };
  const onBack = () => {
    back.popped = true;
    void cancelScan();
  };
  window.addEventListener("popstate", onBack, { once: true });
  setTarget(shown);
  try {
    const cancelled = new Promise<null>((resolve) => {
      settlePending = () => {
        resolve(null);
      };
    });
    const result = await Promise.race([
      scanner.scan({ formats: formats(scanner), windowed: true }),
      cancelled,
    ]);
    return result?.content || null;
  } catch {
    return null; // cancelled or closed
  } finally {
    settlePending = null;
    setTarget(null);
    window.removeEventListener("popstate", onBack);
    // Drop our entry before the caller navigates, so its next step lands on top.
    if (!back.popped) {
      await new Promise<void>((resolve) => {
        window.addEventListener("popstate", () => resolve(), { once: true });
        window.history.back();
      });
    }
  }
}

/** Opens the camera and returns the scanned ISBN, or null if the user cancelled. */
export function scanIsbn(): Promise<string | null> {
  return scan((scanner) => [scanner.Format.EAN13], "barcode");
}

/**
 * Opens the camera and returns a scanned product code (EAN-13, EAN-8, UPC-A or
 * UPC-E), or null if the user cancelled. Used by the add-search step, which
 * looks up whatever code comes back with `lookupCode` (D60).
 */
export function scanCode(): Promise<string | null> {
  return scan(
    (scanner) => [
      scanner.Format.EAN13,
      scanner.Format.EAN8,
      scanner.Format.UPC_A,
      scanner.Format.UPC_E,
    ],
    "barcode",
  );
}

/** Scans a QR code (the pairing code another device shows), or null if cancelled. */
export function scanQr(): Promise<string | null> {
  return scan((scanner) => [scanner.Format.QRCode], "qr");
}
