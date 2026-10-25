// Reading the ISBN printed on a book from a photo (Phase 8), for when its barcode is a shop's label. The photo is
// taken with the phone's own camera app, through a file input with `capture`, and read on the device by Rust.

import { invoke } from "@tauri-apps/api/core";

/**
 * Opens the phone's camera app; the photo in base64, or null if the person came back without one. Call it from the
 * tap itself, before any `await`: the webview only opens a file chooser while the tap lasts. The input stays in
 * the page until the photo has been read, so nothing collects it while the camera app is in front.
 */
export function takePhoto(): Promise<string | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "image/*";
    input.capture = "environment";
    input.hidden = true;
    const done = (photo: string | null) => {
      input.remove();
      resolve(photo);
    };
    input.addEventListener(
      "change",
      () => {
        const file = input.files?.[0];
        if (!file) return done(null);
        // A data URL, read natively: Tauri's calls carry JSON on Android, so the photo goes as base64.
        const reader = new FileReader();
        reader.onload = () => {
          const url = typeof reader.result === "string" ? reader.result : "";
          done(url.slice(url.indexOf(",") + 1));
        };
        reader.onerror = () => done("");
        reader.readAsDataURL(file);
      },
      { once: true },
    );
    input.addEventListener("cancel", () => done(null), { once: true });
    document.body.append(input);
    input.click();
  });
}

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
