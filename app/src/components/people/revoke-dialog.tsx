import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Smartphone, Trash2 } from "lucide-react";

import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { type Person, type PersonDevice, removeDevice, sharingOverview } from "@/lib/api";
import { cn } from "@/lib/utils";

/**
 * A lost or stolen device (Design 15, D58): it stops receiving every library it
 * shared, what it sends no longer counts, and if it connects again those libraries
 * are erased from it.
 */
export function RevokeDialog({
  person,
  device: fixed,
  desktop,
  onClose,
  onDone,
}: {
  /** Whose device it is; `null` closes the dialog. */
  person: Person | null;
  /** On mobile the device is already chosen, from its own menu. */
  device?: PersonDevice | null;
  desktop: boolean;
  onClose: () => void;
  onDone: (message: string) => void;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const library = nameOf(libraries.active);
  const candidates = person?.devices.filter((d) => !d.thisDevice) ?? [];
  const [chosen, setChosen] = useState<string | null>(null);
  const [receives, setReceives] = useState<Record<string, string[]>>({});
  const [busy, setBusy] = useState(false);
  const device = fixed ?? candidates.find((d) => d.id === chosen) ?? candidates[0] ?? null;

  useEffect(() => {
    if (!person) return;
    sharingOverview()
      .then((all) => setReceives(Object.fromEntries(all.map((d) => [d.deviceId, d.libraries]))))
      .catch(() => setReceives({}));
  }, [person]);

  if (!person || !device) return null;
  const names = (receives[device.id] ?? []).map((id) =>
    nameOf(libraries.all.find((l) => l.id === id)),
  );
  const others = person.devices.filter((d) => d.id !== device.id);
  // Rust refuses it (SEG-036): said before, not as an error after.
  const blocked = device.lastDeviceOfLastAdmin;

  const remove = async () => {
    setBusy(true);
    try {
      await removeDevice(device.id);
      onDone(t("people.revoke.done", { device: device.name }));
    } catch (e) {
      onDone(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SheetOrDialog
      open
      desktop={desktop}
      onClose={onClose}
      title={
        fixed
          ? t("people.revoke.titleOne", { device: device.name })
          : t("people.revoke.title", { person: person.name })
      }
      description={t("people.revoke.why")}
      icon={
        <span className="flex size-10 shrink-0 items-center justify-center rounded-full bg-destructive/10 text-destructive">
          <Smartphone className="size-5" />
        </span>
      }
    >
      <p className="text-sm text-muted-foreground">{t("people.revoke.why")}</p>
      {blocked && (
        <div className="flex flex-col gap-2 text-sm leading-relaxed text-pretty">
          <p>{t("people.revoke.lastBody", { person: person.name, library })}</p>
          <p className="text-[13px] text-muted-foreground">{t("people.revoke.lastNote")}</p>
        </div>
      )}
      {!fixed && candidates.length > 1 && (
        <div role="radiogroup" className="flex flex-col divide-y rounded-xl border">
          {candidates.map((d) => (
            <button
              key={d.id}
              type="button"
              role="radio"
              aria-checked={d.id === device.id}
              onClick={() => setChosen(d.id)}
              className={cn(
                "flex min-h-[52px] items-center gap-3 px-3.5 text-left text-sm font-medium",
                d.id === device.id && "bg-primary/8",
              )}
            >
              <span
                className={cn(
                  "size-[18px] shrink-0 rounded-full border-[1.5px] border-muted-foreground",
                  d.id === device.id && "border-[5px] border-primary",
                )}
              />
              {d.name}
            </button>
          ))}
        </div>
      )}
      <div
        className={cn(
          "flex flex-col gap-2 text-sm leading-relaxed text-pretty",
          blocked && "hidden",
        )}
      >
        <p>
          {names.length > 0
            ? t("people.revoke.loses", { libraries: names.join(", ") })
            : t("people.revoke.losesAll")}
        </p>
        <p>{t("people.revoke.role", { person: person.name, library })}</p>
        <p className="text-[13px] text-muted-foreground">
          {t("people.revoke.after")}
          {others[0] &&
            ` ${t("people.revoke.stays", { person: person.name, device: others[0].name })}`}
        </p>
        <p className="text-[13px] text-muted-foreground">
          {t("people.revoke.tip", { person: person.name })}
        </p>
      </div>
      <div className={cn("flex gap-2", desktop ? "justify-end" : "flex-col-reverse")}>
        <Button variant="outline" className={cn(!desktop && "h-12")} onClick={onClose}>
          {blocked ? t("common.close") : t("common.cancel")}
        </Button>
        {!blocked && (
          <Button
            variant="destructive"
            className={cn(!desktop && "h-12")}
            disabled={busy}
            onClick={() => void remove()}
          >
            <Trash2 /> {t("people.revoke.confirm", { device: device.name })}
          </Button>
        )}
      </div>
    </SheetOrDialog>
  );
}
