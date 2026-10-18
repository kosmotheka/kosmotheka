import type { SyncReport } from "@/lib/api";

/** The cloud the app uses unless the person types another (`account::OFFICIAL_SERVER` in Rust). */
const OFFICIAL_SERVER = "https://api.kosmotheka.app";

/** Whether `server` is exactly the official one: a server of one's own, a local test one included, is not. */
export function isOfficial(server: string): boolean {
  return server.trim().replace(/\/+$/, "") === OFFICIAL_SERVER;
}

/** The host of a server address or link ("biblio.casa-ruiz.es"), without the scheme or the path. */
export function hostOf(url: string): string {
  const text = url.trim();
  try {
    const host = new URL(text).host;
    if (host) return host;
  } catch {
    // Not a full address: what the person typed, as it is.
  }
  return text.replace(/^[a-z][a-z0-9+.-]*:\/\//i, "").replace(/[/?#].*$/, "");
}

/** Libraries that came with a session on a cloud that is not the official one (see `SyncReport`). */
export function foreignCloudNotices(
  report: Pick<SyncReport, "foreignClouds">,
  t: (key: string, options: Record<string, string>) => string,
  fallbackName: string,
): string[] {
  return report.foreignClouds.map((found) =>
    t("cloud.official.arrived", {
      name: found.name || fallbackName,
      host: hostOf(found.cloudServer),
    }),
  );
}
