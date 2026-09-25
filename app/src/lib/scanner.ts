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
