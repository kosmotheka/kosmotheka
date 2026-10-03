import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { CircleAlert, CircleCheck, Copy, Loader2, Lock, LogIn } from "lucide-react";

import { Tick } from "@/components/libraries/library-checklist";
import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Button } from "@/components/ui/button";
import { useAccount } from "@/lib/account";
import {
  type LibraryCloudStatus,
  type LibrarySummary,
  cloudErrorKey,
  duplicateLibrary,
  uploadToCloud,
} from "@/lib/api";
import { recordSynced, refreshCloudStatus, useCloud } from "@/lib/cloud";
import { closeOverlay, openOverlay } from "@/lib/navigation";

type Progress = "waiting" | "uploading" | "done" | { failed: string };

/**
 * Which libraries go to the cloud (CloudPick.dc.html). Each one is uploaded on its own, so
 * one failing never stops the rest. A library with unsigned history cannot go up as it is:
 * «Duplicate as new» (D58) makes a signed copy, which is then offered for upload.
 */
export function CloudPickDialog({
  open,
  onDone,
}: {
  open: boolean;
  /** Called with how many libraries went up. */
  onDone: (count: number) => void;
}) {
  const { t } = useTranslation();
  const account = useAccount();
  const cloud = useCloud();
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const [ticked, setTicked] = useState<string[]>([]);
  const [progress, setProgress] = useState<Record<string, Progress> | null>(null);
  const [copied, setCopied] = useState<string | null>(null);
  const [duplicating, setDuplicating] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [seeded, setSeeded] = useState(false);

  // Each time it opens, start from what can go up, all ticked.
  if (open && !seeded) {
    setSeeded(true);
    setProgress(null);
    setCopied(null);
    setError(null);
  } else if (!open && seeded) {
    setSeeded(false);
  }

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    void refreshCloudStatus().then(
      (status) => !cancelled && setTicked(status.filter((s) => s.canUpload).map((s) => s.library)),
    );
    return () => {
      cancelled = true;
    };
  }, [open]);

  const rows = cloud.status.flatMap((status) => {
    const library = libraries.all.find((l) => l.id === status.library);
    return library ? [{ library, status }] : [];
  });
  const uploading = progress !== null && Object.values(progress).some((p) => p === "uploading");
  const finished = progress !== null && !uploading;
  const chosen = ticked.filter((id) =>
    rows.some((r) => r.status.library === id && r.status.canUpload),
  );

  const toggle = (id: string) =>
    setTicked((current) =>
      current.includes(id) ? current.filter((c) => c !== id) : [...current, id],
    );

  const upload = async () => {
    const queue = [...chosen];
    const state: Record<string, Progress> = Object.fromEntries(queue.map((id) => [id, "waiting"]));
    setProgress({ ...state });
    let uploaded = 0;
    for (const id of queue) {
      state[id] = "uploading";
      setProgress({ ...state });
      try {
        recordSynced(await uploadToCloud(id));
        state[id] = "done";
        uploaded += 1;
      } catch (e) {
        state[id] = { failed: cloudErrorKey(e) };
      }
      setProgress({ ...state });
    }
    await refreshCloudStatus();
    libraries.reload();
    if (uploaded > 0) onDone(uploaded);
  };

  const duplicate = async (library: LibrarySummary) => {
    setDuplicating(library.id);
    setError(null);
    try {
      const name = t("libraries.duplicateName", { name: nameOf(library) });
      const id = await duplicateLibrary(library.id, name);
      libraries.reload();
      await refreshCloudStatus();
      setTicked((current) => [...current, id]);
      setCopied(name);
    } catch {
      setError(t("cloud.pick.duplicateFailed"));
    } finally {
      setDuplicating(null);
    }
  };

  const sub = (status: LibraryCloudStatus, library: LibrarySummary) => {
    const works = t("nav.works", { count: library.works });
    if (status.linkedHere) return `${works} · ${t("cloud.pick.inCloud")}`;
    if (status.cloudLibrary) return t("cloud.pick.otherServer");
    return works;
  };

  const rowIcon = (id: string, on: boolean, inCloud: boolean) => {
    const step = progress?.[id];
    if (step === "uploading") return <Loader2 className="size-5 shrink-0 animate-spin" />;
    if (step === "done" || inCloud) return <CircleCheck className="size-5 shrink-0 text-success" />;
    if (step && typeof step === "object")
      return <CircleAlert className="size-5 shrink-0 text-destructive" />;
    return <Tick on={on} />;
  };

  return (
    <ResponsiveDialog
      open={open}
      onOpenChange={(next) => !next && closeOverlay()}
      title={t("cloud.pick.title")}
      className="sm:max-w-lg"
    >
      {!account.signedIn ? (
        <div className="flex flex-col items-start gap-3 rounded-2xl border p-5">
          <p className="text-sm text-pretty text-muted-foreground">{t("cloud.pick.signedOut")}</p>
          <Button onClick={() => openOverlay({ name: "auth" })}>
            <LogIn /> {t("account.settings.signInOrCreate")}
          </Button>
        </div>
      ) : (
        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-1.5">
            {account.user && (
              <span className="flex items-center gap-2 text-sm font-medium text-success">
                <CircleCheck className="size-4" />
                {t("cloud.pick.signedInAs", { name: account.user.name })}
              </span>
            )}
            <p className="text-sm text-pretty text-muted-foreground">{t("cloud.pick.intro")}</p>
          </div>

          {copied && (
            <p className="flex items-start gap-2 rounded-xl bg-secondary px-3.5 py-3 text-sm text-pretty">
              <Copy className="mt-0.5 size-4 shrink-0" />
              {t("cloud.pick.duplicated", { name: copied })}
            </p>
          )}

          <div className="flex flex-col divide-y overflow-hidden rounded-2xl border">
            {rows.map(({ library, status }) => {
              const id = library.id;
              const inCloud = status.linkedHere;
              const step = progress?.[id];
              const failed = step && typeof step === "object" ? step.failed : null;
              const tickable = status.canUpload;
              const body = (
                <>
                  {rowIcon(id, ticked.includes(id), inCloud)}
                  <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                    <span className="truncate text-[15px] font-medium">{nameOf(library)}</span>
                    <span className="text-[13px] text-muted-foreground">
                      {sub(status, library)}
                    </span>
                    {failed && (
                      <span className="text-[13px] text-pretty text-destructive">{t(failed)}</span>
                    )}
                  </span>
                </>
              );
              if (tickable) {
                return (
                  <button
                    key={id}
                    type="button"
                    role="checkbox"
                    aria-checked={ticked.includes(id)}
                    disabled={progress !== null}
                    className="flex min-h-[60px] items-center gap-3.5 px-4 py-2 text-left hover:bg-muted disabled:opacity-100 disabled:hover:bg-transparent"
                    onClick={() => toggle(id)}
                  >
                    {body}
                  </button>
                );
              }
              return (
                <div key={id} className="flex flex-col gap-2 px-4 py-2.5">
                  <div className="flex min-h-[44px] items-center gap-3.5">
                    {inCloud ? (
                      body
                    ) : (
                      <>
                        <Lock className="size-5 shrink-0 text-muted-foreground" />
                        <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                          <span className="truncate text-[15px] font-medium">
                            {nameOf(library)}
                          </span>
                          <span className="text-[13px] text-muted-foreground">
                            {sub(status, library)}
                          </span>
                        </span>
                      </>
                    )}
                  </div>
                  {!inCloud && status.unsignedHistory && (
                    <div className="flex flex-col items-start gap-2.5 pl-[34px]">
                      <p className="text-[13px] text-pretty text-muted-foreground">
                        {t("cloud.pick.unsigned")}
                      </p>
                      <Button
                        variant="outline"
                        size="sm"
                        disabled={duplicating !== null || progress !== null}
                        onClick={() => void duplicate(library)}
                      >
                        <Copy /> {t("cloud.pick.duplicate")}
                      </Button>
                    </div>
                  )}
                  {!inCloud && !status.unsignedHistory && !status.cloudLibrary && (
                    <p className="pl-[34px] text-[13px] text-pretty text-muted-foreground">
                      {t("cloud.pick.locked")}
                    </p>
                  )}
                </div>
              );
            })}
          </div>
          {error && <p className="text-[13px] text-destructive">{error}</p>}

          <div className="flex flex-col gap-2.5">
            {finished ? (
              <Button className="h-12" onClick={closeOverlay}>
                {t("cloud.pick.finish")}
              </Button>
            ) : (
              <>
                <Button
                  className="h-12"
                  disabled={chosen.length === 0 || progress !== null}
                  onClick={() => void upload()}
                >
                  {uploading ? (
                    <>
                      <Loader2 className="animate-spin" /> {t("cloud.pick.uploading")}
                    </>
                  ) : (
                    t("cloud.pick.upload", { count: chosen.length })
                  )}
                </Button>
                <Button
                  variant="ghost"
                  className="h-11"
                  disabled={uploading}
                  onClick={closeOverlay}
                >
                  {t("cloud.pick.notNow")}
                </Button>
              </>
            )}
            <p className="text-center text-[13px] text-pretty text-muted-foreground">
              {t("cloud.pick.later")}
            </p>
          </div>
        </div>
      )}
    </ResponsiveDialog>
  );
}
