import { useState } from "react";
import { useTranslation } from "react-i18next";
import { MonitorSmartphone, TriangleAlert } from "lucide-react";

import { LibraryChecklist } from "@/components/libraries/library-checklist";
import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { type DeviceSharing, setSharing } from "@/lib/api";

/**
 * Which libraries go to one paired device. Unticking one withdraws that device's
 * access: the library disappears from it on the next sync (D71).
 */
export function ShareTargetsDialog({
  device,
  desktop,
  onClose,
  onSaved,
}: {
  device: DeviceSharing | null;
  desktop: boolean;
  onClose: () => void;
  onSaved: (message: string) => void;
}) {
  const { t } = useTranslation();
  const [selected, setSelected] = useState<string[]>([]);
  const [shownFor, setShownFor] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const libraries = useLibraries();
  const nameOf = useLibraryName();

  // Another device opened the dialog: start from what it receives now.
  if (device && shownFor !== device.deviceId) {
    setShownFor(device.deviceId);
    setSelected(device.libraries);
    setError(null);
  }
  if (!device) return null;

  const unticked = device.libraries.filter((id) => !selected.includes(id));
  const ticked = selected.filter((id) => !device.libraries.includes(id));
  const names = (ids: string[]) =>
    ids.map((id) => nameOf(libraries.all.find((library) => library.id === id))).join(", ");

  const close = () => {
    setShownFor(null);
    onClose();
  };

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      for (const id of ticked) await setSharing(id, device.deviceId, true);
      for (const id of unticked) await setSharing(id, device.deviceId, false);
      onSaved(t("sharing.saved", { device: device.name }));
      close();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SheetOrDialog
      open
      desktop={desktop}
      onClose={close}
      title={t("sharing.targetsTitle", { device: device.name })}
      description={t("sharing.targetsHint", { device: device.name })}
      icon={<MonitorSmartphone className="size-5 shrink-0 text-muted-foreground" />}
    >
      <p className="-mt-2 text-sm text-pretty text-muted-foreground">
        {t("sharing.targetsHint", { device: device.name })}
      </p>
      <LibraryChecklist selected={selected} onChange={setSelected} disabled={busy} />
      {unticked.length > 0 && (
        <p className="flex gap-2.5 rounded-xl bg-destructive/10 px-3.5 py-3 text-sm leading-snug">
          <TriangleAlert className="mt-0.5 size-4 shrink-0 text-destructive" />
          <span className="text-pretty">
            {t("sharing.unticked", {
              count: unticked.length,
              names: names(unticked),
              device: device.name,
            })}
          </span>
        </p>
      )}
      {error && <p className="text-[13px] text-destructive">{error}</p>}
      <div className="grid grid-cols-2 gap-2.5">
        <Button variant="outline" className="h-12" onClick={close}>
          {t("form.cancel")}
        </Button>
        <Button
          className="h-12"
          disabled={busy || (ticked.length === 0 && unticked.length === 0)}
          onClick={() => void save()}
        >
          {t("form.save")}
        </Button>
      </div>
    </SheetOrDialog>
  );
}
