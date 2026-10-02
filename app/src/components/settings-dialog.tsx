import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronRight, HandHelping, HardDrive, LogIn, MonitorSmartphone } from "lucide-react";

import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { supportedLanguages } from "@/i18n";
import { useAccount } from "@/lib/account";
import {
  type LibrarySummary,
  giveDeviceAway,
  lanStatus,
  listLibraries,
  setDeviceName,
} from "@/lib/api";
import { openOverlay } from "@/lib/navigation";

/** Each language in its own name, so it can be found whatever the current one. */
const LANGUAGE_NAMES: Record<(typeof supportedLanguages)[number], string> = {
  en: "English",
  es: "Español",
};

export function SettingsDialog({
  open,
  desktop,
  onOpenChange,
}: {
  open: boolean;
  desktop: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t, i18n } = useTranslation();
  const account = useAccount();
  const host = account.official ? "kosmotheka.app" : account.server.replace(/^https?:\/\//, "");

  const [deviceName, setDeviceNameState] = useState("");
  const [editingName, setEditingName] = useState(false);
  const [handingOver, setHandingOver] = useState(false);

  useEffect(() => {
    if (!open) return;
    lanStatus()
      .then((status) => setDeviceNameState(status.deviceName))
      .catch(() => {});
  }, [open]);

  return (
    <>
      <ResponsiveDialog
        open={open}
        onOpenChange={onOpenChange}
        title={t("settings.title")}
        description={t("settings.description")}
      >
        <div className="space-y-6">
          <div className="space-y-2">
            <p className="text-sm font-medium">{t("account.settings.cloudAndAccount")}</p>
            {account.signedIn && account.user ? (
              <button
                type="button"
                className="flex w-full items-center gap-3 rounded-lg border p-3 text-left hover:bg-accent"
                onClick={() => openOverlay({ name: "account" })}
              >
                <span className="flex size-10 shrink-0 items-center justify-center rounded-full bg-secondary text-sm font-semibold">
                  {account.user.name.slice(0, 1).toUpperCase()}
                </span>
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="text-sm font-medium">{account.user.name}</span>
                  <span className="text-sm text-muted-foreground">
                    {t("account.settings.connected", { host })}
                  </span>
                </span>
                <ChevronRight className="size-4 shrink-0 text-muted-foreground" />
              </button>
            ) : (
              <div className="flex flex-col gap-3 rounded-lg border p-3">
                <div className="flex gap-3">
                  <span className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-secondary">
                    <HardDrive className="size-[19px]" />
                  </span>
                  <span className="flex flex-col gap-0.5">
                    <span className="text-sm font-medium">{t("account.settings.noneTitle")}</span>
                    <span className="text-sm text-muted-foreground">
                      {t("account.settings.noneBody")}
                    </span>
                  </span>
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => openOverlay({ name: "auth" })}
                  >
                    <LogIn /> {t("account.settings.signInOrCreate")}
                  </Button>
                  <span className="text-sm text-muted-foreground">
                    {t("account.settings.optionalHint")}
                  </span>
                </div>
              </div>
            )}
          </div>
          <div className="space-y-2">
            <p className="text-sm font-medium">{t("nav.languages")}</p>
            <div className="flex gap-2">
              {supportedLanguages.map((language) => (
                <Button
                  key={language}
                  type="button"
                  size="sm"
                  variant={i18n.resolvedLanguage === language ? "secondary" : "outline"}
                  onClick={() => void i18n.changeLanguage(language)}
                >
                  {LANGUAGE_NAMES[language]}
                </Button>
              ))}
            </div>
          </div>
          <button
            type="button"
            className="flex w-full items-center gap-3 rounded-lg border p-3 text-left hover:bg-accent"
            onClick={() => openOverlay({ name: "providers" })}
          >
            <span className="flex-1">
              <span className="block text-sm font-medium">{t("settings.providers")}</span>
              <span className="block text-sm text-muted-foreground">
                {t("settings.providersHelp")}
              </span>
            </span>
            <ChevronRight className="size-4 shrink-0 text-muted-foreground" />
          </button>

          <div className="space-y-2">
            <p className="text-sm font-medium">{t("account.thisDevice.section")}</p>
            <div className="flex flex-col rounded-lg border">
              <button
                type="button"
                className="flex min-h-[52px] items-center gap-3 border-b px-3 text-left hover:bg-accent"
                onClick={() => setEditingName(true)}
              >
                <MonitorSmartphone className="size-[18px] shrink-0 text-muted-foreground" />
                <span className="flex-1 text-sm">{t("account.thisDevice.name")}</span>
                <span className="text-sm text-muted-foreground">{deviceName}</span>
              </button>
              <button
                type="button"
                className="flex min-h-[60px] items-center gap-3 px-3 py-2 text-left hover:bg-accent"
                onClick={() => setHandingOver(true)}
              >
                <HandHelping className="size-[18px] shrink-0 text-muted-foreground" />
                <span className="flex flex-1 flex-col gap-0.5">
                  <span className="text-sm">{t("account.thisDevice.handOver")}</span>
                  <span className="text-sm text-muted-foreground">
                    {t("account.thisDevice.handOverHint")}
                  </span>
                </span>
                <ChevronRight className="size-4 shrink-0 text-muted-foreground" />
              </button>
            </div>
          </div>

          <div className="flex justify-end">
            <Button type="button" onClick={() => onOpenChange(false)}>
              {t("common.close")}
            </Button>
          </div>
        </div>
      </ResponsiveDialog>

      <EditDeviceNameDialog
        open={editingName}
        desktop={desktop}
        name={deviceName}
        onClose={() => setEditingName(false)}
        onSaved={setDeviceNameState}
      />

      <HandOverConfirm
        open={handingOver}
        desktop={desktop}
        onClose={() => setHandingOver(false)}
      />
    </>
  );
}

function EditDeviceNameDialog({
  open,
  desktop,
  name,
  onClose,
  onSaved,
}: {
  open: boolean;
  desktop: boolean;
  name: string;
  onClose: () => void;
  onSaved: (name: string) => void;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState(name);
  const [busy, setBusy] = useState(false);
  const [seeded, setSeeded] = useState(false);

  if (open && !seeded) {
    setSeeded(true);
    setDraft(name);
  } else if (!open && seeded) {
    setSeeded(false);
  }

  const save = async () => {
    setBusy(true);
    try {
      await setDeviceName(draft.trim());
      onSaved(draft.trim());
      onClose();
    } finally {
      setBusy(false);
    }
  };

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={onClose}
      title={t("account.thisDevice.name")}
      description={t("account.thisDevice.name")}
    >
      <div className="flex flex-col gap-4">
        <Input autoFocus value={draft} onChange={(e) => setDraft(e.target.value)} disabled={busy} />
        <div className="grid grid-cols-2 gap-2.5">
          <Button variant="outline" className="h-11" onClick={onClose}>
            {t("form.cancel")}
          </Button>
          <Button className="h-11" disabled={busy || !draft.trim()} onClick={() => void save()}>
            {t("form.save")}
          </Button>
        </div>
      </div>
    </SheetOrDialog>
  );
}

/** "Dejar este dispositivo a otra persona" (D81, Confirm.dc.html "handOver"). */
function HandOverConfirm({
  open,
  desktop,
  onClose,
}: {
  open: boolean;
  desktop: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const [libraries, setLibraries] = useState<LibrarySummary[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // A short delay before the red action can be tapped, so opening the sheet
  // (e.g. a scroll misfire) never wipes the device by accident.
  const [armed, setArmed] = useState(false);
  const [seeded, setSeeded] = useState(false);

  if (open && !seeded) {
    setSeeded(true);
    setArmed(false);
    setError(null);
    listLibraries()
      .then(setLibraries)
      .catch(() => setLibraries([]));
  } else if (!open && seeded) {
    setSeeded(false);
  }

  useEffect(() => {
    if (!open) return;
    const timer = setTimeout(() => setArmed(true), 3000);
    return () => clearTimeout(timer);
  }, [open]);

  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      await giveDeviceAway();
      // The app reloads from the `device-reset` event (or, in the dev mock,
      // straight from the mock handler); nothing else to do here.
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={onClose}
      title={t("account.confirm.handOver.title")}
      description={t("account.confirm.handOver.title")}
      icon={
        <span className="flex size-11 shrink-0 items-center justify-center rounded-full bg-destructive/15 text-destructive">
          <HandHelping className="size-[21px]" />
        </span>
      }
    >
      <div className="flex flex-col gap-3">
        <p className="text-[15px] text-pretty">{t("account.confirm.handOver.body")}</p>
        {libraries.length > 0 && (
          <div className="flex flex-wrap gap-1.5">
            {libraries.map((library) => (
              <span
                key={library.id}
                className="flex h-7 items-center rounded-lg bg-secondary px-2.5 text-[13px] font-medium"
              >
                {library.name}
              </span>
            ))}
          </div>
        )}
        <Button variant="outline" className="h-11" onClick={() => openOverlay({ name: "backups" })}>
          {t("account.confirm.backupFirst")}
        </Button>
        <p className="text-[13px] text-pretty text-muted-foreground">
          {t("account.confirm.handOver.note")}
        </p>
        {error && <p className="text-[13px] text-destructive">{error}</p>}
        <div className="grid grid-cols-2 gap-2.5 pt-1">
          <Button variant="outline" className="h-12" disabled={busy} onClick={onClose}>
            {t("form.cancel")}
          </Button>
          <Button
            variant="destructive"
            className="h-12"
            disabled={busy || !armed}
            onClick={() => void run()}
          >
            {t("account.confirm.handOver.action")}
          </Button>
        </div>
      </div>
    </SheetOrDialog>
  );
}
