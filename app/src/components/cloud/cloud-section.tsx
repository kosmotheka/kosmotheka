import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Cloud, CloudDownload, CloudOff, RefreshCw, TriangleAlert } from "lucide-react";

import { CloudStateLine, useCloudRows } from "@/components/cloud/cloud-state";
import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { Button } from "@/components/ui/button";
import { type LeftLibrary, cloudErrorKey, leftLibraries, redownloadLibrary } from "@/lib/api";
import { cloudNotices, refreshCloudStatus, runCloudSync, useCloud } from "@/lib/cloud";
import { openOverlay } from "@/lib/navigation";

/**
 * The libraries this device removed by its own choice and the cloud may hand over again: one button each, the only
 * way back (a sync never brings them down). A library whose access was withdrawn is not here, it never comes back.
 */
function LeftLibraries({
  onChanged,
  onNotice,
}: {
  onChanged: () => void;
  onNotice: (message: string) => void;
}) {
  const { t } = useTranslation();
  const cloud = useCloud();
  const libraries = useLibraries();
  const [left, setLeft] = useState<LeftLibrary[]>([]);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    leftLibraries()
      .then((found) => !cancelled && setLeft(found))
      .catch(() => !cancelled && setLeft([]));
    return () => {
      cancelled = true;
    };
  }, [cloud.status]);

  if (left.length === 0) return null;

  const bringDown = async (entry: LeftLibrary) => {
    const name = entry.name || t("nav.myLibrary");
    setBusy(entry.id);
    try {
      const report = await redownloadLibrary(entry.id);
      onNotice(
        report.outcome === "downloaded"
          ? t("cloud.left.back", { name })
          : report.outcome === "missing_in_cloud"
            ? t("cloud.left.missing", { name })
            : report.outcome === "other_server"
              ? t("cloud.left.otherServer", { name })
              : t("cloud.left.withdrawn", { name }),
      );
    } catch (e) {
      onNotice(t(cloudErrorKey(e)));
    } finally {
      setBusy(null);
      libraries.reload();
      await refreshCloudStatus();
      onChanged();
      setLeft(await leftLibraries().catch(() => []));
    }
  };

  return (
    <section className="flex flex-col gap-2.5">
      <div className="flex flex-col gap-0.5">
        <h2 className="text-[15px] font-semibold">{t("cloud.left.title")}</h2>
        <p className="text-[13px] text-pretty text-muted-foreground">{t("cloud.left.hint")}</p>
      </div>
      <div className="flex flex-col divide-y overflow-hidden rounded-2xl border">
        {left.map((entry) => (
          <div key={entry.id} className="flex min-h-16 items-center gap-3 px-3.5 py-2">
            <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-secondary">
              <CloudDownload className="size-[19px]" />
            </span>
            <span className="min-w-0 flex-1 truncate text-[15px] font-medium">
              {entry.name || t("nav.myLibrary")}
            </span>
            <Button
              variant="outline"
              size="sm"
              className="shrink-0"
              disabled={busy !== null}
              onClick={() => void bringDown(entry)}
            >
              {t("cloud.left.bringDown")}
            </Button>
          </div>
        ))}
      </div>
    </section>
  );
}

/**
 * The cloud in «Sharing and devices» (Sharing.dc.html): one row per library with its
 * state, next to the paired devices. Hidden when signed out.
 */
export function CloudSharingSection({
  onChanged,
  onNotice,
}: {
  /** Something arrived from the cloud, so the lists reload. */
  onChanged: () => void;
  onNotice: (message: string) => void;
}) {
  const { t } = useTranslation();
  const cloud = useCloud();
  const rows = useCloudRows();
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const [busy, setBusy] = useState(false);

  if (rows.length === 0) return null;

  const syncNow = async () => {
    setBusy(true);
    const reports = await runCloudSync();
    setBusy(false);
    onChanged();
    if (!reports) return;
    const notices = cloudNotices(reports, libraries.all, t, false);
    const received = reports.reduce((sum, r) => sum + r.received, 0);
    const sent = reports.reduce((sum, r) => sum + r.sent, 0);
    onNotice([t("cloud.sharing.synced", { received, sent }), ...notices].join(" "));
  };

  return (
    <>
      <section className="flex flex-col gap-2.5">
        <div className="flex items-center gap-3">
          <div className="flex min-w-0 flex-1 flex-col gap-0.5">
            <h2 className="text-[15px] font-semibold">{t("cloud.sharing.title")}</h2>
            <p className="text-[13px] text-pretty text-muted-foreground">
              {t("cloud.sharing.hint")}
            </p>
          </div>
          <Button variant="outline" disabled={busy || cloud.syncing} onClick={() => void syncNow()}>
            <RefreshCw className={busy || cloud.syncing ? "animate-spin" : undefined} />
            {t("devices.syncNow")}
          </Button>
        </div>

        {cloud.offline && (
          <div className="flex flex-col gap-1 rounded-xl border border-warning/40 bg-warning-soft px-3.5 py-3 text-sm text-pretty text-foreground">
            <span className="flex items-center gap-2 font-medium">
              <CloudOff className="size-4" /> {t("cloud.errors.offline")}
            </span>
            <span className="text-[13px] text-muted-foreground">
              {t("cloud.sharing.offlineHelp")}
            </span>
          </div>
        )}
        {cloud.errorKey && (
          <p className="flex items-start gap-2 rounded-xl border border-destructive/40 px-3.5 py-3 text-sm text-pretty">
            <TriangleAlert className="mt-0.5 size-4 shrink-0 text-destructive" />
            {t(cloud.errorKey)}
          </p>
        )}

        <div className="flex flex-col divide-y overflow-hidden rounded-2xl border">
          {rows.map((row) => (
            <div key={row.library.id} className="flex min-h-16 items-center gap-3 px-3.5 py-2">
              <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-secondary">
                {row.kind === "offline" ? (
                  <CloudOff className="size-[19px]" />
                ) : (
                  <Cloud className="size-[19px]" />
                )}
              </span>
              <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                <span className="truncate text-[15px] font-medium">{nameOf(row.library)}</span>
                <CloudStateLine row={row} />
              </span>
              {(row.kind === "notUploaded" || row.kind === "unsigned") && (
                <Button
                  variant="outline"
                  size="sm"
                  className="shrink-0"
                  onClick={() => openOverlay({ name: "cloudPick" })}
                >
                  {t(row.kind === "unsigned" ? "cloud.row.fix" : "cloud.row.upload")}
                </Button>
              )}
            </div>
          ))}
        </div>
      </section>
      <LeftLibraries onChanged={onChanged} onNotice={onNotice} />
    </>
  );
}
