import { useState } from "react";
import { useTranslation } from "react-i18next";
import { MonitorSmartphone, Undo2 } from "lucide-react";

import { AuthErrorBlock } from "@/components/account/auth-error-block";
import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { type AccountActionError, accountErrorOf } from "@/lib/account-errors";
import {
  type AccountDevice,
  accountDevices,
  removeAccountDevice,
  restoreAccountDevice,
} from "@/lib/api";
import { closeOverlay } from "@/lib/navigation";

/** "Hace 2 horas" / "Hace 3 días", or the special case for right now. */
function relativeSeen(
  lastSeenAt: string | null,
  locale: string,
  t: (key: string) => string,
): string {
  if (!lastSeenAt) return t("account.devices.never");
  const diffMs = Date.now() - Date.parse(lastSeenAt);
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  const minutes = Math.round(diffMs / 60000);
  if (minutes < 1) return rtf.format(0, "minute");
  if (minutes < 60) return rtf.format(-minutes, "minute");
  const hours = Math.round(minutes / 60);
  if (hours < 24) return rtf.format(-hours, "hour");
  const days = Math.round(hours / 24);
  return rtf.format(-days, "day");
}

/**
 * Ajustes › Cuenta › Mis dispositivos (MyDevices.dc.html): everyone who has
 * signed in with this account, when last seen, and removing or undoing one.
 */
export function DevicesDialog({ open, desktop }: { open: boolean; desktop: boolean }) {
  const { t, i18n } = useTranslation();
  const [devices, setDevices] = useState<AccountDevice[] | null>(null);
  const [loadError, setLoadError] = useState<AccountActionError | null>(null);
  const [seeded, setSeeded] = useState(false);
  const [removing, setRemoving] = useState<AccountDevice | null>(null);
  const [undoError, setUndoError] = useState<AccountActionError | null>(null);

  const load = () => {
    accountDevices()
      .then((loaded) => {
        setDevices(loaded);
        setLoadError(null);
      })
      .catch((e: unknown) => setLoadError(accountErrorOf(e)));
  };

  if (open && !seeded) {
    setSeeded(true);
    load();
  } else if (!open && seeded) {
    setSeeded(false);
  }

  const undo = async (device: AccountDevice) => {
    setUndoError(null);
    try {
      await restoreAccountDevice(device.id);
      load();
    } catch (e) {
      setUndoError(accountErrorOf(e));
    }
  };

  const confirmRemove = async () => {
    if (!removing) return;
    await removeAccountDevice(removing.id);
    setRemoving(null);
    load();
  };

  const locale = i18n.resolvedLanguage ?? "en";

  return (
    <>
      <ResponsiveDialog
        open={open}
        onOpenChange={(next) => !next && closeOverlay()}
        title={t("account.devices.title")}
      >
        <div className="flex flex-col gap-3.5">
          {loadError && (
            <AuthErrorBlock
              reason={loadError.reason}
              retryAfter={loadError.retryAfter}
              onAction={load}
            />
          )}
          {undoError && (
            <AuthErrorBlock reason={undoError.reason} retryAfter={undoError.retryAfter} />
          )}
          <p className="text-sm text-pretty text-muted-foreground">{t("account.devices.intro")}</p>
          <div className="flex flex-col rounded-2xl border">
            {(devices ?? []).map((device) => (
              <div
                key={device.id}
                className={
                  "flex min-h-[68px] items-center gap-3 border-b px-4 py-2.5 last:border-b-0" +
                  (device.removed ? " bg-muted" : "")
                }
              >
                <span
                  className={
                    "flex size-10 shrink-0 items-center justify-center rounded-xl " +
                    (device.removed
                      ? "border border-dashed text-muted-foreground"
                      : "bg-secondary text-secondary-foreground")
                  }
                >
                  <MonitorSmartphone className="size-[19px]" />
                </span>
                <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                  <span
                    className={
                      "flex items-center gap-2 text-[15px] font-semibold" +
                      (device.removed ? " text-muted-foreground line-through" : "")
                    }
                  >
                    {device.name}
                    {device.current && (
                      <span className="rounded-md bg-secondary px-1.5 py-0.5 text-[11px] font-semibold text-secondary-foreground">
                        {t("account.devices.current")}
                      </span>
                    )}
                  </span>
                  <span className="text-[13px] text-pretty text-muted-foreground">
                    {device.removed
                      ? t(
                          device.canRestore
                            ? "account.devices.removedPending"
                            : "account.devices.removedFinal",
                        )
                      : device.current
                        ? t("account.devices.nowCurrent")
                        : relativeSeen(device.lastSeenAt, locale, t)}
                  </span>
                </span>
                {!device.removed && !device.current && (
                  <Button variant="outline" onClick={() => setRemoving(device)}>
                    {t("account.devices.remove")}
                  </Button>
                )}
                {device.removed && device.canRestore && (
                  <Button
                    variant="ghost"
                    className="text-primary"
                    onClick={() => void undo(device)}
                  >
                    <Undo2 /> {t("account.devices.undo")}
                  </Button>
                )}
              </div>
            ))}
          </div>
        </div>
      </ResponsiveDialog>

      <SheetOrDialog
        open={removing !== null}
        desktop={desktop}
        onClose={() => setRemoving(null)}
        title={t("account.confirm.removeDevice.title", { name: removing?.name ?? "" })}
        description={t("account.confirm.removeDevice.title", { name: removing?.name ?? "" })}
        icon={
          <span className="flex size-11 shrink-0 items-center justify-center rounded-full bg-destructive/15 text-destructive">
            <MonitorSmartphone className="size-[21px]" />
          </span>
        }
      >
        <div className="flex flex-col gap-3">
          <p className="text-[15px] text-pretty">{t("account.confirm.removeDevice.body")}</p>
          <div
            role="status"
            className="flex gap-3 rounded-xl bg-warning-soft p-3.5 text-sm text-pretty text-warning"
          >
            {t("account.confirm.removeDevice.warn")}
          </div>
          <p className="text-[13px] text-pretty text-muted-foreground">
            {t("account.confirm.removeDevice.note")}
          </p>
          <div className="grid grid-cols-2 gap-2.5 pt-1">
            <Button variant="outline" className="h-12" onClick={() => setRemoving(null)}>
              {t("form.cancel")}
            </Button>
            <Button variant="destructive" className="h-12" onClick={() => void confirmRemove()}>
              {t("account.confirm.removeDevice.action")}
            </Button>
          </div>
        </div>
      </SheetOrDialog>
    </>
  );
}
