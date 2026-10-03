import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Fingerprint, LibraryBig } from "lucide-react";

import { Button } from "@/components/ui/button";
import { appLockAfter, appLockEnabled } from "@/lib/api";
import { lockAvailable, unlock } from "@/lib/app-lock";

/** `?lock` in the dev mock renders the screen straight away for screenshots. */
function mockForced(): boolean {
  return import.meta.env.DEV && new URLSearchParams(window.location.search).has("lock");
}

/**
 * Covers the whole app, over any open dialog, until the user passes the system
 * prompt (D78). It is plain state here, not a navigation overlay: the Android
 * back button must never "go back" past the lock and reveal the content.
 */
export function AppLockScreen() {
  const { t } = useTranslation();
  // `null` while the setting is read: the content stays covered, so it never flashes
  // before the lock appears.
  const [locked, setLocked] = useState<boolean | null>(() => (mockForced() ? true : null));
  // While the system prompt itself is up, Android can report the app as hidden;
  // that must not be read as "went to the background".
  const authenticating = useRef(false);
  const hiddenAt = useRef<number | null>(null);

  const attempt = async (useDeviceCredential: boolean) => {
    authenticating.current = true;
    const ok = await unlock(useDeviceCredential);
    authenticating.current = false;
    if (ok) setLocked(false);
  };

  // On start: lock only if the setting is on AND biometrics still work. If they
  // no longer do, the user would be shut out of their own data, so open normally.
  useEffect(() => {
    // An object, not a `let`: TypeScript cannot see the cleanup change a local across the await.
    const run = { cancelled: false };
    void (async () => {
      const [enabled, available] = await Promise.all([appLockEnabled(), lockAvailable()]).catch(
        (): [boolean, boolean] => [false, false],
      );
      if (run.cancelled) return;
      if (!enabled || !available) {
        setLocked((current) => current ?? false);
        return;
      }
      setLocked(true);
      void attempt(false);
    })();
    return () => {
      run.cancelled = true;
    };
  }, []);

  useEffect(() => {
    const onVisibility = () => {
      if (document.visibilityState === "hidden") {
        hiddenAt.current = Date.now();
        return;
      }
      if (authenticating.current) return;
      const hiddenFor = hiddenAt.current ? Date.now() - hiddenAt.current : 0;
      hiddenAt.current = null;
      if (hiddenFor === 0) return;
      void (async () => {
        const [enabled, available, minutes] = await Promise.all([
          appLockEnabled(),
          lockAvailable(),
          appLockAfter(),
        ]);
        if (!enabled || !available || hiddenFor <= minutes * 60_000) return;
        setLocked(true);
        void attempt(false);
      })();
    };
    document.addEventListener("visibilitychange", onVisibility);
    return () => document.removeEventListener("visibilitychange", onVisibility);
  }, []);

  if (locked === false) return null;
  if (locked === null) return <div className="fixed inset-0 z-[100] bg-background" />;

  return (
    <div className="fixed inset-0 z-[100] flex flex-col items-center justify-center gap-6 bg-background px-7 text-center">
      <div className="flex items-center gap-2.5">
        <span className="flex size-10 items-center justify-center rounded-[11px] bg-foreground text-background">
          <LibraryBig className="size-[21px]" />
        </span>
        <span className="text-[19px] font-semibold tracking-tight">Kosmotheka</span>
      </div>
      <button
        type="button"
        aria-label={t("appLock.screen.unlock")}
        className="my-2 flex size-[88px] items-center justify-center rounded-full border bg-secondary text-primary"
        onClick={() => void attempt(false)}
      >
        <Fingerprint className="size-10" />
      </button>
      <div className="flex flex-col gap-2.5">
        <h1 className="text-[28px] font-semibold tracking-tight">{t("appLock.screen.title")}</h1>
        <p className="text-base text-muted-foreground">{t("appLock.screen.body")}</p>
      </div>
      <div className="flex w-full max-w-[340px] flex-col gap-3 pt-2">
        <Button className="h-12" onClick={() => void attempt(false)}>
          {t("appLock.screen.unlock")}
        </Button>
        <Button variant="outline" className="h-12" onClick={() => void attempt(true)}>
          {t("appLock.screen.useDevicePin")}
        </Button>
      </div>
    </div>
  );
}
