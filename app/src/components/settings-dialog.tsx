import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ChevronRight,
  Fingerprint,
  HandHelping,
  HardDrive,
  LogIn,
  MailOpen,
  MonitorSmartphone,
  UserRound,
} from "lucide-react";

import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { ProvidersDialog } from "@/components/providers-dialog";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { supportedLanguages } from "@/i18n";
import { useAccount } from "@/lib/account";
import {
  type LibrarySummary,
  appLockAfter,
  appLockEnabled,
  giveDeviceAway,
  isMobile,
  lanStatus,
  listLibraries,
  setAppLock,
  setAppLockAfter,
  personName,
  setDeviceName,
  setPersonName,
} from "@/lib/api";
import { lockAvailable, unlock } from "@/lib/app-lock";
import { openOverlay } from "@/lib/navigation";
import { setThemeChoice, themeChoice, themeChoices } from "@/lib/theme";

/** Mirrors `settings::APP_LOCK_AFTER_CHOICES`, in minutes. */
const LOCK_AFTER_CHOICES = [0, 5, 60, 720, 1440] as const;

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
  const [person, setPerson] = useState<string | null>(null);
  const [editingPerson, setEditingPerson] = useState(false);
  const [handingOver, setHandingOver] = useState(false);

  const [mobile, setMobile] = useState(false);
  const [lockOk, setLockOk] = useState(false);
  const [lockOn, setLockOn] = useState(false);
  const [lockAfter, setLockAfterState] = useState(60);
  const [lockBusy, setLockBusy] = useState(false);
  const [theme, setTheme] = useState(themeChoice);
  // Opened over Settings, so closing it comes back here, as from About.
  const [providers, setProviders] = useState(false);

  useEffect(() => {
    if (!open) return;
    lanStatus()
      .then((status) => setDeviceNameState(status.deviceName))
      .catch(() => {});
    void personName().then(setPerson);
    void isMobile().then(setMobile);
    void lockAvailable().then(setLockOk);
    void appLockEnabled().then(setLockOn);
    void appLockAfter().then(setLockAfterState);
  }, [open]);

  // Turning it on asks for a successful prompt first, so nobody locks themselves
  // out; turning it off needs no prompt (D78).
  const toggleLock = async (next: boolean) => {
    if (next) {
      setLockBusy(true);
      const ok = await unlock(false);
      setLockBusy(false);
      if (!ok) return;
    }
    await setAppLock(next);
    setLockOn(next);
  };

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
                  <Button variant="outline" size="sm" onClick={() => openOverlay({ name: "auth" })}>
                    <LogIn /> {t("account.settings.signInOrCreate")}
                  </Button>
                  <span className="text-sm text-muted-foreground">
                    {t("account.settings.optionalHint")}
                  </span>
                </div>
              </div>
            )}
            <Button
              variant="ghost"
              size="sm"
              className="self-start"
              onClick={() => openOverlay({ name: "invitation" })}
            >
              <MailOpen /> {t("invitations.open.entry")}
            </Button>
          </div>
          <div className="space-y-2">
            <p className="text-sm font-medium">{t("nav.languages")}</p>
            <div className="flex gap-2">
              {supportedLanguages.map((language) => (
                <Button
                  key={language}
                  type="button"
                  size="sm"
                  variant={i18n.resolvedLanguage === language ? "default" : "outline"}
                  aria-pressed={i18n.resolvedLanguage === language}
                  onClick={() => void i18n.changeLanguage(language)}
                >
                  {LANGUAGE_NAMES[language]}
                </Button>
              ))}
            </div>
          </div>
          <div className="space-y-2">
            <p className="text-sm font-medium">{t("settings.theme.title")}</p>
            <div className="flex gap-2">
              {themeChoices.map((choice) => (
                <Button
                  key={choice}
                  type="button"
                  size="sm"
                  variant={theme === choice ? "default" : "outline"}
                  aria-pressed={theme === choice}
                  onClick={() => {
                    setThemeChoice(choice);
                    setTheme(choice);
                  }}
                >
                  {t(`settings.theme.${choice}`)}
                </Button>
              ))}
            </div>
          </div>
          <button
            type="button"
            className="flex w-full items-center gap-3 rounded-lg border p-3 text-left hover:bg-accent"
            onClick={() => setProviders(true)}
          >
            <span className="flex-1">
              <span className="block text-sm font-medium">{t("settings.providers")}</span>
              <span className="block text-sm text-muted-foreground">
                {t("settings.providersHelp")}
              </span>
            </span>
            <ChevronRight className="size-4 shrink-0 text-muted-foreground" />
          </button>

          {mobile && (
            <div className="space-y-2">
              <p className="text-sm font-medium">{t("settings.security.title")}</p>
              <div className="flex items-center gap-3 rounded-lg border p-3">
                <Fingerprint className="size-[18px] shrink-0 text-muted-foreground" />
                <span className="flex flex-1 flex-col gap-0.5">
                  <span className="text-sm">{t("settings.security.toggle")}</span>
                  <span className="text-sm text-muted-foreground">
                    {lockOk
                      ? t("settings.security.toggleHelp")
                      : t("settings.security.unavailable")}
                  </span>
                </span>
                <Switch
                  checked={lockOn}
                  disabled={!lockOk || lockBusy}
                  onCheckedChange={(checked) => void toggleLock(checked)}
                />
              </div>
              {lockOn && lockOk && (
                <div className="space-y-2 pt-1">
                  <p className="text-sm text-muted-foreground">{t("settings.security.after")}</p>
                  <div className="flex flex-wrap gap-2">
                    {LOCK_AFTER_CHOICES.map((minutes) => (
                      <Button
                        key={minutes}
                        type="button"
                        size="sm"
                        variant={lockAfter === minutes ? "default" : "outline"}
                        aria-pressed={lockAfter === minutes}
                        onClick={() =>
                          void setAppLockAfter(minutes).then(() => setLockAfterState(minutes))
                        }
                      >
                        {t(`settings.security.afterChoice.${minutes}`)}
                      </Button>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}

          <div className="space-y-2">
            <p className="text-sm font-medium">{t("account.thisDevice.section")}</p>
            <div className="flex flex-col rounded-lg border">
              <button
                type="button"
                className="flex min-h-[52px] items-center gap-3 border-b px-3 text-left hover:bg-accent"
                onClick={() => setEditingPerson(true)}
              >
                <UserRound className="size-[18px] shrink-0 text-muted-foreground" />
                <span className="flex flex-1 flex-col gap-0.5">
                  <span className="text-sm">{t("settings.person.title")}</span>
                  <span className="text-xs text-muted-foreground">{t("settings.person.hint")}</span>
                </span>
                <span className="text-sm text-muted-foreground">{person}</span>
              </button>
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

      <EditNameDialog
        open={editingName}
        desktop={desktop}
        title={t("account.thisDevice.name")}
        name={deviceName}
        save={async (name) => {
          await setDeviceName(name);
          return name;
        }}
        onClose={() => setEditingName(false)}
        onSaved={setDeviceNameState}
      />
      <EditNameDialog
        open={editingPerson}
        desktop={desktop}
        title={t("settings.person.title")}
        description={t("settings.person.hint")}
        name={person ?? ""}
        save={setPersonName}
        onClose={() => setEditingPerson(false)}
        onSaved={setPerson}
      />

      <HandOverConfirm open={handingOver} desktop={desktop} onClose={() => setHandingOver(false)} />
      <ProvidersDialog open={providers} onOpenChange={setProviders} />
    </>
  );
}

/** One name to edit: the device's, or the person's. `save` returns the name as kept. */
function EditNameDialog({
  open,
  desktop,
  title,
  description,
  name,
  save: persist,
  onClose,
  onSaved,
}: {
  open: boolean;
  desktop: boolean;
  title: string;
  description?: string;
  name: string;
  save: (name: string) => Promise<string>;
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
      onSaved(await persist(draft.trim()));
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
      title={title}
      description={description ?? title}
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
