import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Archive, CircleCheck, LogOut, MonitorSmartphone, Trash2 } from "lucide-react";
import { save } from "@tauri-apps/plugin-dialog";

import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import {
  type KnownDevice,
  deleteLibrary,
  exportBackup,
  leaveLibrary,
  libraryDevices,
} from "@/lib/api";
import { closeOverlay, navigate, openOverlay } from "@/lib/navigation";

const today = () => new Date().toISOString().slice(0, 10);

function useDevices(open: boolean, id: string | undefined) {
  const [devices, setDevices] = useState<KnownDevice[]>([]);
  useEffect(() => {
    if (!open || !id) return;
    let cancelled = false;
    libraryDevices(id)
      .then((found) => !cancelled && setDevices(found))
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [open, id]);
  return devices;
}

/**
 * Deleting a library everywhere cannot be undone (D65): the dialog says so, names
 * the devices it will vanish from, and puts saving a backup first.
 */
export function DeleteLibraryDialog({
  open,
  desktop,
  onDone,
}: {
  open: boolean;
  desktop: boolean;
  /** Called with the deleted library's name and the one now active. */
  onDone: (deleted: string, active: string) => void;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const library = libraries.active;
  const devices = useDevices(open, library?.id);
  const [savedTo, setSavedTo] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!library) return null;
  const name = nameOf(library);

  const close = () => {
    setSavedTo(null);
    setError(null);
    closeOverlay();
  };

  const backUp = async () => {
    setError(null);
    const path = await save({
      defaultPath: `${name.toLocaleLowerCase().replace(/\s+/g, "-")}-${today()}.zip`,
      filters: [{ name: t("data.backupFile"), extensions: ["zip"] }],
    });
    if (!path) return;
    setBusy(true);
    try {
      await exportBackup(path, library.id);
      setSavedTo(path);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    setBusy(true);
    try {
      const active = await deleteLibrary(library.id);
      setSavedTo(null);
      libraries.reload();
      onDone(name, active);
      navigate("catalog");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={close}
      title={t("libraries.deleteTitle", { name })}
      description={t("libraries.deleteWarning", { count: library.works })}
      icon={
        <span className="flex size-11 shrink-0 items-center justify-center rounded-full bg-destructive/15 text-destructive">
          <Trash2 className="size-[22px]" />
        </span>
      }
    >
      <div className="flex flex-col gap-2.5 text-[15px] leading-normal text-pretty">
        <p>
          <strong className="font-semibold">{t("libraries.cannotUndo")}</strong>{" "}
          {t(devices.length > 0 ? "libraries.deleteWarningShared" : "libraries.deleteWarning", {
            count: library.works,
          })}
        </p>
        {devices.length > 0 && (
          <div className="flex flex-wrap gap-1.5">
            {devices.map((device) => (
              <span
                key={device.deviceId}
                className="flex h-8 items-center gap-1.5 rounded-lg bg-secondary px-2.5 text-[13px] font-medium"
              >
                <MonitorSmartphone className="size-[15px]" /> {device.name}
              </span>
            ))}
          </div>
        )}
      </div>
      {savedTo ? (
        <div className="flex items-center gap-3 rounded-2xl bg-success-soft px-4 py-3.5 text-success">
          <CircleCheck className="size-[22px] shrink-0" />
          <span className="flex min-w-0 flex-col gap-0.5">
            <span className="text-[15px] font-semibold">{t("libraries.backupSaved")}</span>
            <span className="truncate text-[13px] text-foreground">{savedTo}</span>
          </span>
        </div>
      ) : (
        <div className="flex flex-col gap-3 rounded-2xl bg-muted p-4">
          <span className="text-sm leading-snug text-pretty">
            {t("libraries.backupFirst", { name })}
          </span>
          <Button className="h-12" disabled={busy} onClick={() => void backUp()}>
            <Archive /> {t("libraries.saveBackupOf", { name })}
          </Button>
        </div>
      )}
      {error && <p className="text-[13px] text-destructive">{error}</p>}
      <div className="grid grid-cols-2 gap-2.5">
        <Button variant="outline" className="h-12" onClick={close}>
          {t("form.cancel")}
        </Button>
        {savedTo ? (
          <Button variant="destructive" className="h-12" disabled={busy} onClick={() => void remove()}>
            {t("libraries.deleteNamed", { name })}
          </Button>
        ) : (
          <Button
            variant="outline"
            className="h-12 border-destructive text-destructive"
            disabled={busy}
            onClick={() => void remove()}
          >
            {t("libraries.deleteWithoutBackup")}
          </Button>
        )}
      </div>
    </SheetOrDialog>
  );
}

/** Leaving only removes the library from this device; the others keep it. */
export function LeaveLibraryDialog({
  open,
  desktop,
  onDone,
}: {
  open: boolean;
  desktop: boolean;
  onDone: (left: string, active: string) => void;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const library = libraries.active;
  const devices = useDevices(open, library?.id);
  const [busy, setBusy] = useState(false);

  if (!library) return null;
  const name = nameOf(library);

  const leave = async () => {
    setBusy(true);
    try {
      const active = await leaveLibrary(library.id);
      libraries.reload();
      onDone(name, active);
      navigate("catalog");
    } finally {
      setBusy(false);
    }
  };

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={closeOverlay}
      title={t("libraries.leaveTitle", { name })}
      description={t("libraries.leaveHint")}
      icon={
        <span className="flex size-11 shrink-0 items-center justify-center rounded-full bg-secondary">
          <LogOut className="size-[21px]" />
        </span>
      }
    >
      <p className="text-[15px] leading-normal text-pretty">
        {devices.length > 0
          ? t("libraries.leaveKeptIn", { name, devices: devices.map((d) => d.name).join(", ") })
          : t("libraries.leaveOnlyHere", { name })}
      </p>
      {devices.length > 0 && (
        <p className="-mt-2 text-[13px] text-pretty text-muted-foreground">
          {t("libraries.leaveComeBack")}
        </p>
      )}
      <div className="grid grid-cols-2 gap-2.5">
        <Button variant="outline" className="h-12" onClick={closeOverlay}>
          {t("form.cancel")}
        </Button>
        {devices.length > 0 ? (
          <Button className="h-12" disabled={busy} onClick={() => void leave()}>
            {t("libraries.leaveNamed", { name })}
          </Button>
        ) : (
          // Only here, leaving would lose it for good: deleting offers a backup first.
          <Button
            variant="outline"
            className="h-12 border-destructive text-destructive"
            onClick={() => openOverlay({ name: "deleteLibrary" })}
          >
            {t("libraries.goToDelete")}
          </Button>
        )}
      </div>
    </SheetOrDialog>
  );
}
