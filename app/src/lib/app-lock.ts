// Fingerprint/face unlock (D78). This is a screen lock, not encryption: the plugin
// exists only in phone builds, so it is loaded on demand and never touched on
// desktop or in the `?mock` preview (see `lib/scanner.ts` for the same pattern).

import i18n from "@/i18n";
import { isMobile } from "@/lib/api";

/** Whether the system can currently show the fingerprint/face prompt on this device. */
export async function lockAvailable(): Promise<boolean> {
  if (!(await isMobile())) return false;
  try {
    const biometric = await import("@tauri-apps/plugin-biometric");
    const status = await biometric.checkStatus();
    return status.isAvailable;
  } catch {
    return false;
  }
}

/**
 * Shows the system prompt. `useDeviceCredential` also accepts the phone's PIN or
 * pattern, for the lock screen's "Use your phone's PIN" button. Resolves to whether
 * the user got in; a cancel or failure resolves to `false`, it never throws.
 */
export async function unlock(useDeviceCredential: boolean): Promise<boolean> {
  try {
    const biometric = await import("@tauri-apps/plugin-biometric");
    await biometric.authenticate(i18n.t("appLock.reason"), {
      allowDeviceCredential: useDeviceCredential,
      title: i18n.t("appLock.title"),
      subtitle: i18n.t("appLock.subtitle"),
      cancelTitle: i18n.t("appLock.cancel"),
    });
    return true;
  } catch {
    return false;
  }
}
