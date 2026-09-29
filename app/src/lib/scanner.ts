// Camera barcode scanning (D35). The plugin exists only in phone builds, so it is
// loaded on demand and never touched on desktop.

export class CameraDenied extends Error {}

/** Opens the camera and returns the scanned ISBN, or null if the user cancelled. */
export async function scanIsbn(): Promise<string | null> {
  const scanner = await import("@tauri-apps/plugin-barcode-scanner");
  let permission = await scanner.checkPermissions();
  if (permission !== "granted") permission = await scanner.requestPermissions();
  if (permission !== "granted") throw new CameraDenied();
  try {
    const result = await scanner.scan({
      formats: [scanner.Format.EAN13],
      windowed: false,
    });
    return result.content || null;
  } catch {
    return null; // cancelled or closed
  }
}

/**
 * Opens the camera and returns a scanned product code (EAN-13, EAN-8, UPC-A or
 * UPC-E), or null if the user cancelled. Used by the add-search step, which
 * looks up whatever code comes back with `lookupCode` (D60).
 */
export async function scanCode(): Promise<string | null> {
  const scanner = await import("@tauri-apps/plugin-barcode-scanner");
  let permission = await scanner.checkPermissions();
  if (permission !== "granted") permission = await scanner.requestPermissions();
  if (permission !== "granted") throw new CameraDenied();
  try {
    const result = await scanner.scan({
      formats: [
        scanner.Format.EAN13,
        scanner.Format.EAN8,
        scanner.Format.UPC_A,
        scanner.Format.UPC_E,
      ],
      windowed: false,
    });
    return result.content || null;
  } catch {
    return null; // cancelled or closed
  }
}

/** Scans a QR code (the pairing code another device shows), or null if cancelled. */
export async function scanQr(): Promise<string | null> {
  const scanner = await import("@tauri-apps/plugin-barcode-scanner");
  let permission = await scanner.checkPermissions();
  if (permission !== "granted") permission = await scanner.requestPermissions();
  if (permission !== "granted") throw new CameraDenied();
  try {
    const result = await scanner.scan({ formats: [scanner.Format.QRCode], windowed: false });
    return result.content || null;
  } catch {
    return null;
  }
}
