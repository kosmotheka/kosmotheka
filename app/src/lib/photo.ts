// A photo or an image for the app to read or keep: taken with the phone's own camera app, through a file input
// with `capture`, or chosen among the device's images. It goes to Rust in base64: on Android, Tauri's calls carry
// JSON.

/**
 * Opens the phone's camera app (or, with `camera: false`, the device's images); the image in base64, or null if the
 * person came back without one. Call it from the tap itself, before any `await`: the webview only opens a file
 * chooser while the tap lasts. The input stays in the page until the image has been read, so nothing collects it
 * while the camera app is in front.
 */
export function takePhoto({ camera = true }: { camera?: boolean } = {}): Promise<string | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "image/*";
    if (camera) input.capture = "environment";
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
