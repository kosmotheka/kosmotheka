// Reading the ISBN printed on a book from a photo (Phase 8), for when its barcode is a shop's label. The photo is
// taken with the phone's own camera app, through a file input with `capture`, and read on the device by Rust.

import { invoke } from "@tauri-apps/api/core";

/**
 * The ISBNs printed in a photo from `takePhoto` (those the check digit confirms), or null if the person took none.
 * An empty list is a photo where no ISBN could be read, or that could not be read at all.
 */
export async function isbnsInPhoto(taking: Promise<string | null>): Promise<string[] | null> {
  const photo = await taking;
  if (photo === null) return null;
  try {
    // Sent as taken: shrinking it in the webview first takes longer than reading it whole (13 s to encode a
    // 4080 × 3072 photo on a moto g84, against about 2 s for ML Kit to read it).
    return await invoke<string[]>("isbns_in_photo", { photo });
  } catch {
    return [];
  }
}
