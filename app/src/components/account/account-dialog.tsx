import { type ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Check,
  ChevronRight,
  Cloud,
  KeyRound,
  LogOut,
  MailOpen,
  MonitorSmartphone,
  ShieldCheck,
} from "lucide-react";

import { AuthErrorBlock } from "@/components/account/auth-error-block";
import { SecondFactorInput } from "@/components/account/second-factor-input";
import { EditFieldDialog } from "@/components/account/edit-field-dialog";
import {
  type ProofNeeds,
  ProofFields,
  emptyProof,
  proofComplete,
  toActionProof,
} from "@/components/account/proof-fields";
import { Field } from "@/components/book-form";
import { useCloudRows } from "@/components/cloud/cloud-state";
import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { supportedLanguages } from "@/i18n";
import { refreshAccount, useAccount } from "@/lib/account";
import { type AccountActionError, accountErrorOf } from "@/lib/account-errors";
import {
  type AccountDetails,
  type ActionProof,
  type ServerInfo,
  accountDevices,
  accountServerInfo,
  changePassword,
  deleteAccount,
  getAccount,
  regenerateBackupCodes,
  signOut,
  twoFactorDisable,
  updateAccount,
} from "@/lib/api";
import { closeOverlay, openOverlay } from "@/lib/navigation";
import { isSecondFactorCode } from "@/lib/second-factor-code";

/** The host part only ("biblio.casa-ruiz.es", not the scheme), as AuthDialog shows it. */
function hostOf(url: string): string {
  return url.replace(/^https?:\/\//, "").replace(/\/+$/, "");
}

type TwofaAction = "disable" | "regenerate";

/**
 * Ajustes › Cuenta (Account.dc.html): name, email (read-only, D77), language,
 * server, password, second factor, devices, sign out and deleting the account.
 */
export function AccountDialog({ open, desktop }: { open: boolean; desktop: boolean }) {
  const { t, i18n } = useTranslation();
  const account = useAccount();
  const cloudRows = useCloudRows();

  const [details, setDetails] = useState<AccountDetails | null>(null);
  const [loadError, setLoadError] = useState<AccountActionError | null>(null);
  const [twoFactorSupported, setTwoFactorSupported] = useState(true);
  const [deviceCount, setDeviceCount] = useState<number | null>(null);
  const [seeded, setSeeded] = useState(false);

  const [editingName, setEditingName] = useState(false);
  const [editingLanguage, setEditingLanguage] = useState(false);
  const [changingPassword, setChangingPassword] = useState(false);
  const [signingOut, setSigningOut] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [twofaAction, setTwofaAction] = useState<TwofaAction | null>(null);

  const load = () => {
    getAccount()
      .then((loaded) => {
        setDetails(loaded);
        setLoadError(null);
      })
      .catch((e: unknown) => setLoadError(accountErrorOf(e)));
    accountDevices()
      .then((devices) => setDeviceCount(devices.filter((d) => !d.removed).length))
      .catch(() => setDeviceCount(null));
    accountServerInfo()
      .then((info: ServerInfo) => setTwoFactorSupported(info.twoFactor))
      .catch(() => setTwoFactorSupported(true));
  };

  if (open && !seeded) {
    setSeeded(true);
    load();
  } else if (!open && seeded) {
    setSeeded(false);
  }

  // The cached account while `get_account` loads, or when the server can't be
  // reached: the screen still shows something useful instead of a blank state.
  const name = details?.name ?? account.user?.name ?? "";
  const email = details?.email ?? account.user?.email ?? "";
  const locale = details?.locale ?? account.user?.locale ?? i18n.resolvedLanguage ?? "en";
  const host = account.official ? "kosmotheka.app" : hostOf(account.server);

  /** Nombre / Email / Idioma rows: a label column, then the value. */
  const fieldRow = (label: string, value: string, onClick?: () => void) => (
    <div
      role={onClick ? "button" : undefined}
      tabIndex={onClick ? 0 : undefined}
      onClick={onClick}
      onKeyDown={(e) => {
        if (onClick && (e.key === "Enter" || e.key === " ")) onClick();
      }}
      className={
        "flex min-h-[50px] items-center gap-3 border-b px-4 last:border-b-0" +
        (onClick ? " cursor-pointer hover:bg-accent/50" : "")
      }
    >
      <span className="w-20 shrink-0 text-sm text-muted-foreground">{label}</span>
      <span className="flex-1 truncate text-[15px]">{value}</span>
      {onClick && <ChevronRight className="size-[18px] shrink-0 text-muted-foreground" />}
    </div>
  );

  /** "Cambiar la contraseña": an icon, a single line of text, then a chevron. */
  const actionRow = (icon: ReactNode, label: string, onClick: () => void) => (
    <button
      type="button"
      onClick={onClick}
      className="flex min-h-[50px] items-center gap-3 border-b px-4 text-left last:border-b-0 hover:bg-accent/50"
    >
      {icon}
      <span className="flex-1 text-[15px]">{label}</span>
      <ChevronRight className="size-[18px] shrink-0 text-muted-foreground" />
    </button>
  );

  const saveName = async (value: string) => {
    const updated = await updateAccount({ name: value });
    setDetails(updated);
    await refreshAccount();
  };

  const saveLanguage = async (value: string) => {
    const updated = await updateAccount({ locale: value });
    setDetails(updated);
    await refreshAccount();
    setEditingLanguage(false);
  };

  const openDevices = () => openOverlay({ name: "accountDevices" });
  const openTwoFactorSetup = () => openOverlay({ name: "twoFactor" });

  const doSignOut = async () => {
    await signOut();
    await refreshAccount();
    closeOverlay();
  };

  // Until the account is loaded, what the screen shows is what an account with a password has: the usual case.
  const hasPassword = details?.hasPassword !== false;
  const twoFactorOn = details?.twoFactorEnabled === true;

  const doDelete = async (proof: ActionProof) => {
    await deleteAccount(proof);
    await refreshAccount();
    closeOverlay();
  };

  const doTwofaDisable = async (password: string | null, code: string) => {
    await twoFactorDisable(password, code);
    setTwofaAction(null);
    load();
  };

  const doTwofaRegenerate = async (password: string | null, code: string) => {
    const codes = await regenerateBackupCodes(password, code);
    setTwofaAction(null);
    openOverlay({ name: "twoFactor", codes });
  };

  return (
    <>
      <ResponsiveDialog
        open={open}
        onOpenChange={(next) => !next && closeOverlay()}
        title={t("account.screen.title")}
      >
        <div className="flex flex-col gap-5">
          {loadError && (
            <AuthErrorBlock
              reason={loadError.reason}
              retryAfter={loadError.retryAfter}
              onAction={loadError.reason === "server_unavailable" ? load : undefined}
            />
          )}
          <section className="flex flex-col rounded-2xl border">
            {fieldRow(t("account.screen.name"), name, () => setEditingName(true))}
            {fieldRow(t("account.screen.email"), email)}
            {fieldRow(t("account.screen.language"), locale === "es" ? "Español" : "English", () =>
              setEditingLanguage(true),
            )}
            <div className="flex min-h-[50px] items-center gap-3 px-4">
              <span className="w-20 shrink-0 text-sm text-muted-foreground">
                {t("account.screen.server")}
              </span>
              <span className="flex flex-1 items-center gap-1.5 truncate text-[15px]">
                <span className="size-1.5 shrink-0 rounded-full bg-success" />
                {host}
              </span>
            </div>
          </section>

          <section className="flex flex-col gap-2">
            <h2 className="px-1 text-[13px] font-semibold text-muted-foreground">
              {t("account.screen.security")}
            </h2>
            <div className="flex flex-col rounded-2xl border">
              {hasPassword &&
                actionRow(
                  <KeyRound className="size-[18px] shrink-0" />,
                  t("account.screen.changePassword"),
                  () => setChangingPassword(true),
                )}
              <div className="flex flex-col gap-3 border-t px-4 py-3.5 first:border-t-0">
                <div className="flex items-center gap-3">
                  <ShieldCheck className="size-[18px] shrink-0" />
                  <span className="flex flex-1 flex-col gap-0.5">
                    <span className="text-[15px]">{t("account.screen.twoFactor")}</span>
                    {details?.twoFactorEnabled ? (
                      <span className="flex items-center gap-1.5 text-[13px] text-muted-foreground">
                        <span className="size-1.5 shrink-0 rounded-full bg-success" />
                        {t("account.screen.twoFactorOn", { count: details.backupCodesLeft })}
                      </span>
                    ) : (
                      <span className="text-[13px] text-muted-foreground">
                        {t("account.screen.twoFactorOff")}
                      </span>
                    )}
                  </span>
                  {!details?.twoFactorEnabled && twoFactorSupported && (
                    <Button variant="outline" size="sm" onClick={openTwoFactorSetup}>
                      {t("account.screen.turnOn")}
                    </Button>
                  )}
                </div>
                {details?.twoFactorEnabled && (
                  <div className="flex gap-2 pl-[30px]">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setTwofaAction("regenerate")}
                    >
                      {t("account.screen.newCodes")}
                    </Button>
                    <Button variant="outline" size="sm" onClick={() => setTwofaAction("disable")}>
                      {t("account.screen.turnOff")}
                    </Button>
                  </div>
                )}
              </div>
            </div>
          </section>

          <button
            type="button"
            onClick={openDevices}
            className="flex min-h-14 items-center gap-3 rounded-2xl border px-4 text-left hover:bg-accent/50"
          >
            <MonitorSmartphone className="size-[18px] shrink-0" />
            <span className="flex flex-1 flex-col gap-0.5">
              <span className="text-[15px]">{t("account.screen.myDevices")}</span>
              {deviceCount !== null && (
                <span className="text-[13px] text-muted-foreground">
                  {t("account.screen.devicesCount", { count: deviceCount })}
                </span>
              )}
            </span>
            <ChevronRight className="size-[18px] shrink-0 text-muted-foreground" />
          </button>

          <button
            type="button"
            onClick={() => openOverlay({ name: "cloudPick" })}
            className="flex min-h-14 items-center gap-3 rounded-2xl border px-4 text-left hover:bg-accent/50"
          >
            <Cloud className="size-[18px] shrink-0" />
            <span className="flex flex-1 flex-col gap-0.5">
              <span className="text-[15px]">{t("cloud.account.title")}</span>
              {cloudRows.length > 0 && (
                <span className="text-[13px] text-muted-foreground">
                  {t("cloud.account.count", {
                    inCloud: cloudRows.filter((r) => r.status.linkedHere).length,
                    total: cloudRows.length,
                  })}
                </span>
              )}
            </span>
            <ChevronRight className="size-[18px] shrink-0 text-muted-foreground" />
          </button>

          <button
            type="button"
            onClick={() => openOverlay({ name: "invitation" })}
            className="flex min-h-14 items-center gap-3 rounded-2xl border px-4 text-left hover:bg-accent/50"
          >
            <MailOpen className="size-[18px] shrink-0" />
            <span className="flex flex-1 flex-col gap-0.5">
              <span className="text-[15px]">{t("invitations.open.entry")}</span>
              <span className="text-[13px] text-muted-foreground">
                {t("invitations.open.entryHint")}
              </span>
            </span>
            <ChevronRight className="size-[18px] shrink-0 text-muted-foreground" />
          </button>

          <div className="flex flex-col gap-3 pt-2">
            <div className="flex flex-col gap-1.5">
              <Button variant="outline" className="h-12" onClick={() => setSigningOut(true)}>
                <LogOut /> {t("account.screen.signOut")}
              </Button>
              <span className="text-center text-[13px] text-muted-foreground">
                {t("account.screen.signOutHint")}
              </span>
            </div>
            <button
              type="button"
              className="h-11 text-sm font-medium text-destructive"
              onClick={() => setDeleting(true)}
            >
              {t("account.screen.deleteAccount")}
            </button>
          </div>
        </div>
      </ResponsiveDialog>

      <EditFieldDialog
        open={editingName}
        desktop={desktop}
        onOpenChange={setEditingName}
        title={t("account.editName.title")}
        label={t("account.editName.label")}
        value={name}
        onSave={saveName}
      />

      <SheetOrDialog
        open={editingLanguage}
        desktop={desktop}
        onClose={() => setEditingLanguage(false)}
        title={t("account.editLanguage.title")}
        description={t("account.editLanguage.title")}
      >
        <div className="flex gap-2">
          {supportedLanguages.map((lang) => (
            <Button
              key={lang}
              variant={locale === lang ? "secondary" : "outline"}
              className="h-11 flex-1"
              onClick={() => void saveLanguage(lang)}
            >
              {lang === "en" ? "English" : "Español"}
            </Button>
          ))}
        </div>
      </SheetOrDialog>

      <ChangePasswordDialog
        open={changingPassword}
        desktop={desktop}
        onOpenChange={setChangingPassword}
        twoFactor={twoFactorOn}
        onChanged={changePassword}
      />

      <SignOutConfirm
        open={signingOut}
        desktop={desktop}
        onOpenChange={setSigningOut}
        onConfirm={doSignOut}
      />

      <DeleteAccountConfirm
        open={deleting}
        desktop={desktop}
        onOpenChange={setDeleting}
        host={host}
        deviceCount={deviceCount}
        needs={{ who: hasPassword ? "password" : "email", secondFactor: twoFactorOn }}
        onConfirm={doDelete}
      />

      <TwofaOffConfirm
        action={twofaAction}
        hasPassword={hasPassword}
        desktop={desktop}
        onOpenChange={(open) => !open && setTwofaAction(null)}
        onConfirm={twofaAction === "regenerate" ? doTwofaRegenerate : doTwofaDisable}
      />
    </>
  );
}

/** "Cambiar la contraseña": current + new password, then a short success state. */
function ChangePasswordDialog({
  open,
  desktop,
  onOpenChange,
  twoFactor,
  onChanged,
}: {
  open: boolean;
  desktop: boolean;
  onOpenChange: (open: boolean) => void;
  /** The account has the second factor on: its code is asked too (D100). */
  twoFactor: boolean;
  onChanged: (current: string, next: string, totpCode: string | null) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [totpCode, setTotpCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AccountActionError | null>(null);
  const [done, setDone] = useState(false);
  const [seeded, setSeeded] = useState(false);

  if (open && !seeded) {
    setSeeded(true);
    setCurrent("");
    setNext("");
    setTotpCode("");
    setError(null);
    setDone(false);
  } else if (!open && seeded) {
    setSeeded(false);
  }

  const close = () => onOpenChange(false);

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      await onChanged(current, next, twoFactor ? totpCode : null);
      setDone(true);
    } catch (e) {
      setError(accountErrorOf(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={close}
      title={t("account.changePassword.title")}
      description={t("account.changePassword.title")}
    >
      {done ? (
        <div className="flex flex-col items-center gap-4 text-center">
          <span className="flex size-14 items-center justify-center rounded-full bg-success-soft text-success">
            <Check className="size-[26px]" />
          </span>
          <div className="flex flex-col gap-1.5">
            <h3 className="text-lg font-semibold">{t("account.changePassword.successTitle")}</h3>
            <p className="text-[15px] text-pretty text-muted-foreground">
              {t("account.changePassword.successBody")}
            </p>
          </div>
          <Button className="h-11 w-full" onClick={close}>
            {t("common.close")}
          </Button>
        </div>
      ) : (
        <div className="flex flex-col gap-4">
          <Field id="current-password" label={t("account.changePassword.current")}>
            <Input
              id="current-password"
              type="password"
              autoComplete="current-password"
              value={current}
              onChange={(e) => setCurrent(e.target.value)}
              disabled={busy}
            />
          </Field>
          <Field id="new-password" label={t("account.changePassword.new")}>
            <Input
              id="new-password"
              type="password"
              autoComplete="new-password"
              value={next}
              onChange={(e) => setNext(e.target.value)}
              disabled={busy}
            />
          </Field>
          {twoFactor && (
            <Field id="change-password-app-code" label={t("account.confirm.appCodeLabel")}>
              <SecondFactorInput
                id="change-password-app-code"
                value={totpCode}
                onChange={setTotpCode}
                disabled={busy}
                invalid={Boolean(error)}
              />
            </Field>
          )}
          <div className="flex gap-3 rounded-xl bg-muted p-3.5 text-sm text-pretty">
            <LogOut className="mt-0.5 size-[18px] shrink-0" />
            <span>{t("account.changePassword.warning")}</span>
          </div>
          {error && <AuthErrorBlock reason={error.reason} retryAfter={error.retryAfter} />}
          <Button
            className="h-12"
            disabled={
              busy || !current || next.length < 8 || (twoFactor && !isSecondFactorCode(totpCode))
            }
            onClick={() => void save()}
          >
            {t("account.changePassword.save")}
          </Button>
        </div>
      )}
    </SheetOrDialog>
  );
}

function SignOutConfirm({
  open,
  desktop,
  onOpenChange,
  onConfirm,
}: {
  open: boolean;
  desktop: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => Promise<void>;
}) {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AccountActionError | null>(null);

  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      await onConfirm();
    } catch (e) {
      setError(accountErrorOf(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={() => onOpenChange(false)}
      title={t("account.confirm.signOut.title")}
      description={t("account.confirm.signOut.title")}
      icon={
        <span className="flex size-11 shrink-0 items-center justify-center rounded-full bg-secondary text-secondary-foreground">
          <LogOut className="size-[21px]" />
        </span>
      }
    >
      <div className="flex flex-col gap-3">
        <p className="text-[15px] text-pretty">{t("account.confirm.signOut.body")}</p>
        <p className="text-[13px] text-pretty text-muted-foreground">
          {t("account.confirm.signOut.note")}
        </p>
        {error && <AuthErrorBlock reason={error.reason} retryAfter={error.retryAfter} />}
        <div className="grid grid-cols-2 gap-2.5 pt-1">
          <Button variant="outline" className="h-12" onClick={() => onOpenChange(false)}>
            {t("form.cancel")}
          </Button>
          <Button className="h-12" disabled={busy} onClick={() => void run()}>
            {t("account.confirm.signOut.action")}
          </Button>
        </div>
      </div>
    </SheetOrDialog>
  );
}

function DeleteAccountConfirm({
  open,
  desktop,
  onOpenChange,
  host,
  deviceCount,
  needs,
  onConfirm,
}: {
  open: boolean;
  desktop: boolean;
  onOpenChange: (open: boolean) => void;
  host: string;
  deviceCount: number | null;
  /** The password, or a code from the email for an account with none; and the second factor's code if it has it (D100). */
  needs: ProofNeeds;
  onConfirm: (proof: ActionProof) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [proof, setProof] = useState(emptyProof);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AccountActionError | null>(null);
  const [seeded, setSeeded] = useState(false);

  if (open && !seeded) {
    setSeeded(true);
    setProof(emptyProof);
    setError(null);
  } else if (!open && seeded) {
    setSeeded(false);
  }

  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      await onConfirm(toActionProof(proof, needs));
    } catch (e) {
      setError(accountErrorOf(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={() => onOpenChange(false)}
      title={t("account.confirm.deleteAccount.title")}
      description={t("account.confirm.deleteAccount.title")}
      icon={
        <span className="flex size-11 shrink-0 items-center justify-center rounded-full bg-destructive/15 text-destructive">
          <KeyRound className="size-[21px]" />
        </span>
      }
    >
      <div className="flex flex-col gap-3">
        <p className="text-[15px] text-pretty">
          {deviceCount !== null
            ? t("account.confirm.deleteAccount.body", { host, count: deviceCount })
            : t("account.confirm.deleteAccount.bodyNoCount", { host })}
        </p>
        <Button variant="outline" className="h-11" onClick={() => openOverlay({ name: "backups" })}>
          {t("account.confirm.backupFirst")}
        </Button>
        <ProofFields
          idPrefix="delete-account"
          needs={needs}
          proof={proof}
          onChange={setProof}
          disabled={busy}
          invalid={Boolean(error)}
        />
        {error && <AuthErrorBlock reason={error.reason} retryAfter={error.retryAfter} />}
        <div className="grid grid-cols-2 gap-2.5 pt-1">
          <Button variant="outline" className="h-12" onClick={() => onOpenChange(false)}>
            {t("form.cancel")}
          </Button>
          <Button
            variant="destructive"
            className="h-12"
            disabled={busy || !proofComplete(proof, needs)}
            onClick={() => void run()}
          >
            {t("account.confirm.deleteAccount.action")}
          </Button>
        </div>
      </div>
    </SheetOrDialog>
  );
}

function TwofaOffConfirm({
  action,
  hasPassword,
  desktop,
  onOpenChange,
  onConfirm,
}: {
  action: TwofaAction | null;
  /** An account made with Google has none: the code of the second factor is then the whole proof (D100). */
  hasPassword: boolean;
  desktop: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: (password: string | null, code: string) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AccountActionError | null>(null);
  const [seeded, setSeeded] = useState(false);
  const open = action !== null;

  if (open && !seeded) {
    setSeeded(true);
    setPassword("");
    setCode("");
    setError(null);
  } else if (!open && seeded) {
    setSeeded(false);
  }

  const prefix =
    action === "regenerate" ? "account.confirm.twofaRegenerate" : "account.confirm.twofaOff";
  const titleKey = `${prefix}.title`;
  const bodyKey = `${prefix}.body`;
  const noteKey = `${prefix}.note`;
  const actionKey = `${prefix}.action`;

  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      await onConfirm(hasPassword ? password : null, code);
    } catch (e) {
      setError(accountErrorOf(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={() => onOpenChange(false)}
      title={t(titleKey)}
      description={t(titleKey)}
      icon={
        <span className="flex size-11 shrink-0 items-center justify-center rounded-full bg-destructive/15 text-destructive">
          <ShieldCheck className="size-[21px]" />
        </span>
      }
    >
      <div className="flex flex-col gap-3">
        <p className="text-[15px] text-pretty">{t(bodyKey)}</p>
        {hasPassword && (
          <Field id="twofa-off-password" label={t("account.confirm.passwordLabel")}>
            <Input
              id="twofa-off-password"
              type="password"
              autoComplete="current-password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              disabled={busy}
            />
          </Field>
        )}
        <Field id="twofa-off-code" label={t("account.confirm.appCodeLabel")}>
          <SecondFactorInput
            id="twofa-off-code"
            value={code}
            onChange={setCode}
            disabled={busy}
            invalid={Boolean(error)}
          />
        </Field>
        <p className="text-[13px] text-pretty text-muted-foreground">{t(noteKey)}</p>
        {error && <AuthErrorBlock reason={error.reason} retryAfter={error.retryAfter} />}
        <div className="grid grid-cols-2 gap-2.5 pt-1">
          <Button variant="outline" className="h-12" onClick={() => onOpenChange(false)}>
            {t("form.cancel")}
          </Button>
          <Button
            variant="destructive"
            className="h-12"
            disabled={busy || (hasPassword && !password) || !isSecondFactorCode(code)}
            onClick={() => void run()}
          >
            {t(actionKey)}
          </Button>
        </div>
      </div>
    </SheetOrDialog>
  );
}
