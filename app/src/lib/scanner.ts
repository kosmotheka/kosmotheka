// Camera barcode scanning (D35). The plugin exists only in phone builds, so it is
// loaded on demand and never touched on desktop.
//
// The plugin's full-screen camera has no close button and ignores the back
// button, so it runs "windowed": the camera is drawn behind a transparent
// WebView, and `ScannerOverlay` draws the frame and the close button on top.
// Back closes it too, through a history entry of its own.

import type { Format } from "@tauri-apps/plugin-barcode-scanner";
import { useSyncExternalStore } from "react";

import { takePhoto } from "@/lib/photo";
import { isbnsInPhoto } from "@/lib/printed-isbn";

export class CameraDenied extends Error {}

/** What the overlay shows while the camera is open; null when it is closed. */
export type ScanTarget = "barcode" | "qr";

let target: ScanTarget | null = null;
/** Set when the person closed the camera to type the number instead. */
let typing = false;
const listeners = new Set<() => void>();

function notify() {
  listeners.forEach((listener) => listener());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function setTarget(next: ScanTarget | null) {
  target = next;
  document.documentElement.classList.toggle("scanning", next !== null);
  notify();
}

export function useScanTarget(): ScanTarget | null {
  return useSyncExternalStore(subscribe, () => target);
}

/** Closes the camera so the person can type the number. */
export async function typeInstead() {
  typing = true;
  await cancelScan();
}

/** The photo of the printed number the person is taking instead of scanning, if so. */
let photo: Promise<string | null> | null = null;
/** Set when that photo had no ISBN that could be read. */
let noIsbnInPhoto = false;

/**
 * Opens the camera app for a photo of the number printed on the book, and closes the scanner. Called from the tap:
 * the camera app has to be asked for before anything is awaited (see `takePhoto`).
 */
export async function photoInstead() {
  photo = takePhoto();
  await cancelScan();
}

/** Set when that photo had more than one ISBN: the person picks theirs. */
let isbnChoices: string[] | null = null;

/** Once after a photo of the printed number had several ISBNs (another edition's, the ebook's): which one. */
export function readIsbnChoices(): string[] | null {
  const choices = isbnChoices;
  isbnChoices = null;
  return choices;
}

/** True once after a photo of the printed number gave no ISBN: the caller offers to type it. */
export function readNoIsbnInPhoto(): boolean {
  const was = noIsbnInPhoto;
  noIsbnInPhoto = false;
  return was;
}

/** The scanned code or, when the person asked for a photo instead, the first ISBN read in it. */
async function orFromPhoto(code: string | null): Promise<string | null> {
  const taking = photo;
  photo = null;
  if (code !== null || !taking) return code;
  const found = await isbnsInPhoto(taking);
  if (found === null) return null;
  if (found.length === 0) noIsbnInPhoto = true;
  if (found.length > 1) {
    isbnChoices = found;
    return null;
  }
  return found[0] ?? null;
}

/** True once after the person chose to type instead of scanning. */
export function wantsToType(): boolean {
  const wanted = typing;
  typing = false;
  return wanted;
}

// Settles the pending scan from our side. The Android plugin's `cancel` clears
// its saved call before rejecting it, so the plugin's `scan` never settles.
let settlePending: (() => void) | null = null;

/**
 * Closes the camera; the pending scan resolves to null. The scan is settled first, so nothing waits on the
 * plugin's `cancel` while the camera app is in front (`photoInstead`).
 */
export async function cancelScan() {
  settlePending?.();
  const scanner = await import("@tauri-apps/plugin-barcode-scanner");
  await scanner.cancel();
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

/** Set when the last scan read a code that is not an ISBN, such as a shop's price label. */
let notIsbn = false;

/**
 * Opens the camera and returns the scanned ISBN, or null if the user cancelled or the
 * code was not an ISBN (see `readNotIsbn`). Scanning again right away would keep reading
 * the same big label, so the caller offers to type the small printed number instead.
 */
export async function scanIsbn(): Promise<string | null> {
  typing = false;
  notIsbn = false;
  photo = null;
  noIsbnInPhoto = false;
  isbnChoices = null;
  const code = await orFromPhoto(await scan((scanner) => [scanner.Format.EAN13], "barcode"));
  if (code === null || /^97[89]/.test(code)) return code;
  notIsbn = true;
  return null;
}

/** True once after `scanIsbn` read a code that is not an ISBN. */
export function readNotIsbn(): boolean {
  const was = notIsbn;
  notIsbn = false;
  return was;
}

/**
 * Opens the camera and returns a scanned product code (EAN-13, EAN-8, UPC-A or
 * UPC-E), or null if the user cancelled. Used by the add-search step, which
 * looks up whatever code comes back with `lookupCode` (D60).
 */
export async function scanCode(): Promise<string | null> {
  photo = null;
  noIsbnInPhoto = false;
  isbnChoices = null;
  return orFromPhoto(
    await scan(
      (scanner) => [
        scanner.Format.EAN13,
        scanner.Format.EAN8,
        scanner.Format.UPC_A,
        scanner.Format.UPC_E,
      ],
      "barcode",
    ),
  );
}

/** Scans a QR code (the pairing code another device shows), or null if cancelled. */
export function scanQr(): Promise<string | null> {
  return scan((scanner) => [scanner.Format.QRCode], "qr");
}
