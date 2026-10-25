import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Radio } from "lucide-react";

import { useAccount } from "@/lib/account";
import { lanStatus } from "@/lib/api";
import { useCloud } from "@/lib/cloud";
import { navigate } from "@/lib/navigation";
import { cn } from "@/lib/utils";

type Status =
  | { kind: "local" }
  | { kind: "synced"; at: Date; now: number }
  | { kind: "never" }
  | { kind: "offline" };

/** Recent enough to call it "up to date". */
const FRESH_MS = 10 * 60 * 1000;

/**
 * Sync state with the paired devices and the cloud; `version` reloads it after a sync.
 * Not reaching the cloud shows as «offline» but never hides that the LAN works.
 */
export function useSyncStatus(version: number): Status | null {
  const cloud = useCloud();
  const [lan, setLan] = useState<{ count: number; times: number[] } | null>(null);
  const [now, setNow] = useState(() => Date.now());
  // "Synced 5 minutes ago" has to keep moving while the app stays open.
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 60_000);
    return () => clearInterval(timer);
  }, []);
  useEffect(() => {
    let cancelled = false;
    lanStatus()
      .then((status) => {
        if (cancelled) return;
        const devices = [...status.servers, ...status.clients];
        const times = devices
          .map((d) => d.lastSyncAt)
          .filter((at): at is string => at !== null)
          .map((at) => new Date(at).getTime());
        setLan({ count: devices.length, times });
      })
      .catch(() => !cancelled && setLan(null));
    return () => {
      cancelled = true;
    };
  }, [version]);

  if (!lan) return null;
  const linked = cloud.status.filter((s) => s.linkedHere && s.signedIn);
  if (cloud.offline && linked.length > 0) return { kind: "offline" };
  if (lan.count + linked.length === 0) return { kind: "local" };
  const times = [
    ...lan.times,
    ...linked.map((s) => cloud.syncedAt[s.library]).filter((at): at is number => Boolean(at)),
  ];
  return times.length
    ? { kind: "synced", at: new Date(Math.max(...times)), now }
    : { kind: "never" };
}

/** `short` is what fits in the desktop top bar: a word or two, the full text in its tooltip. */
function useLabel(status: Status | null) {
  const { t, i18n } = useTranslation();
  if (!status) return null;
  if (status.kind === "local")
    return { text: t("sync.local"), short: t("sync.short.local"), tone: "muted" as const };
  if (status.kind === "offline")
    return { text: t("sync.offline"), short: t("sync.short.offline"), tone: "warning" as const };
  if (status.kind === "never")
    return { text: t("sync.never"), short: t("sync.short.never"), tone: "warning" as const };
  const age = status.now - status.at.getTime();
  if (age < FRESH_MS) {
    const text = t("sync.upToDate");
    return { text, short: text, tone: "success" as const };
  }
  const minutes = Math.round(age / 60000);
  const rtf = new Intl.RelativeTimeFormat(i18n.resolvedLanguage, { numeric: "auto" });
  const narrow = new Intl.RelativeTimeFormat(i18n.resolvedLanguage, {
    numeric: "auto",
    style: "narrow",
  });
  const ago = (format: Intl.RelativeTimeFormat) =>
    minutes < 60
      ? format.format(-minutes, "minute")
      : minutes < 60 * 48
        ? format.format(-Math.round(minutes / 60), "hour")
        : format.format(-Math.round(minutes / 1440), "day");
  return {
    text: t("sync.lastSync", { when: ago(rtf) }),
    short: ago(narrow),
    tone: "warning" as const,
  };
}

const DOT = { muted: "bg-muted-foreground/60", success: "bg-success", warning: "bg-warning" };

/**
 * Connected to the server's hub or not (D75): small, and only when there is a hub to be
 * connected to. Not live is not an error: the app still syncs on open and after changes.
 */
function LiveMark() {
  const { t } = useTranslation();
  const { signedIn } = useAccount();
  const { live, available } = useCloud();
  if (!signedIn || !available) return null;
  const text = t(live ? "sync.live" : "sync.notLive");
  return (
    <span
      className={cn("flex items-center", live ? "text-success" : "text-muted-foreground/70")}
      title={t(live ? "sync.liveHint" : "sync.notLiveHint")}
      role="img"
      aria-label={text}
    >
      <Radio className="size-3.5" aria-hidden />
    </span>
  );
}

/** A dot and a short text. As a button, it opens "Sharing and devices". */
export function SyncStatus({
  status,
  asButton = false,
  compact = false,
  className,
}: {
  status: Status | null;
  asButton?: boolean;
  /** One line, a word or two: the desktop top bar. */
  compact?: boolean;
  className?: string;
}) {
  const { t } = useTranslation();
  const label = useLabel(status);
  if (!label) return null;
  const content = (
    <>
      <span className={cn("size-1.5 shrink-0 rounded-full", DOT[label.tone])} aria-hidden />
      {compact ? label.short : label.text}
      <LiveMark />
    </>
  );
  const classes = cn(
    "flex items-center gap-1.5 text-xs text-muted-foreground",
    compact && "shrink-0 whitespace-nowrap",
    className,
  );
  if (!asButton) return <span className={classes}>{content}</span>;
  return (
    <button
      type="button"
      className={cn(classes, "h-8 rounded-md px-2.5 text-[13px] hover:bg-muted")}
      aria-label={t("sync.label", { status: label.text })}
      title={label.text}
      onClick={() => navigate("devices")}
    >
      {content}
    </button>
  );
}
