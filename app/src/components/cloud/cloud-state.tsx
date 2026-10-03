import { useTranslation } from "react-i18next";

import { useLibraries } from "@/components/libraries-context";
import type { LibraryCloudStatus, LibrarySummary } from "@/lib/api";
import { type CloudState, useCloud } from "@/lib/cloud";
import { cn } from "@/lib/utils";

/** What to say about one library's cloud, and whether the person can act on it. */
type CloudRowKind =
  | "upToDate"
  | "linked"
  | "syncing"
  | "offline"
  | "error"
  | "missing"
  | "otherServer"
  | "conflict"
  | "notUploaded"
  | "unsigned"
  | "locked";

export interface CloudRow {
  library: LibrarySummary;
  status: LibraryCloudStatus;
  kind: CloudRowKind;
  /** Last sync of this session, for «up to date» and «offline». */
  syncedAt: number | null;
}

const DOT: Record<CloudRowKind, string> = {
  upToDate: "bg-success",
  linked: "bg-success",
  syncing: "bg-muted-foreground/60",
  offline: "bg-warning",
  error: "bg-destructive",
  missing: "bg-warning",
  otherServer: "bg-muted-foreground/60",
  conflict: "bg-warning",
  notUploaded: "bg-muted-foreground/60",
  unsigned: "bg-warning",
  locked: "bg-muted-foreground/60",
};

function kindOf(status: LibraryCloudStatus, cloud: CloudState): CloudRowKind {
  const outcome = cloud.outcomes[status.library];
  if (status.linkedHere) {
    if (outcome === "missing_in_cloud") return "missing";
    if (cloud.offline) return "offline";
    if (cloud.errorKey) return "error";
    if (cloud.syncedAt[status.library]) return "upToDate";
    return cloud.syncing ? "syncing" : "linked";
  }
  if (status.cloudLibrary) return "otherServer";
  if (outcome === "library_conflict") return "conflict";
  if (status.canUpload) return "notUploaded";
  if (status.unsignedHistory) return "unsigned";
  return "locked";
}

/** The cloud state of every library, once signed in; empty otherwise. */
export function useCloudRows(): CloudRow[] {
  const cloud = useCloud();
  const libraries = useLibraries();
  return cloud.status.flatMap((status) => {
    const library = libraries.all.find((l) => l.id === status.library);
    if (!library || !status.signedIn) return [];
    return [
      {
        library,
        status,
        kind: kindOf(status, cloud),
        syncedAt: cloud.syncedAt[status.library] ?? null,
      },
    ];
  });
}

/** «3 minutes ago», from a moment in this session. */
function ago(at: number, locale: string | undefined): string {
  const minutes = Math.max(0, Math.round((Date.now() - at) / 60000));
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  return minutes < 60
    ? rtf.format(-minutes, "minute")
    : minutes < 60 * 48
      ? rtf.format(-Math.round(minutes / 60), "hour")
      : rtf.format(-Math.round(minutes / 1440), "day");
}

/** A dot and one line: «Up to date · 2 minutes ago», «Offline», «Not in the cloud»… */
export function CloudStateLine({ row, className }: { row: CloudRow; className?: string }) {
  const { t, i18n } = useTranslation();
  const when = row.syncedAt ? ago(row.syncedAt, i18n.resolvedLanguage) : null;
  const text =
    row.kind === "upToDate"
      ? t("cloud.row.upToDate", { when })
      : row.kind === "offline"
        ? when
          ? t("cloud.row.offlineSince", { when })
          : t("cloud.row.offline")
        : t(`cloud.row.${row.kind}`);
  return (
    <span className={cn("flex items-center gap-1.5 text-[13px] text-muted-foreground", className)}>
      <span className={cn("size-1.5 shrink-0 rounded-full", DOT[row.kind])} aria-hidden />
      {text}
    </span>
  );
}
