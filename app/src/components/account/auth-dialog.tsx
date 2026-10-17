import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Globe, Info, LogOut, MailOpen, Server } from "lucide-react";

import { AuthErrorBlock } from "@/components/account/auth-error-block";
import { CodeInput } from "@/components/account/code-input";
import { SecondFactorInput } from "@/components/account/second-factor-input";
import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useAccount, refreshAccount } from "@/lib/account";
import {
  type ServerInfo,
  accountServerInfo,
  cancelGoogle,
  confirmAccountEmail,
  finishGoogle,
  forgotPassword,
  isCommandError,
  registerAccount,
  resendAccountCode,
  resetPassword,
  setAccountServer,
  signIn,
  startGoogle,
} from "@/lib/api";
import { GOOGLE_RESULT } from "@/lib/arrived-link";
import { closeOverlay, openOverlay } from "@/lib/navigation";
import { isSecondFactorCode } from "@/lib/second-factor-code";
import { supportedLanguages } from "@/i18n";

type Step =
  | "signin"
  | "server"
  | "signup"
  | "verify"
  | "google"
  | "totp"
  | "backup"
  | "forgot"
  | "recoverCode"
  | "newPass"
  | "recoverTotp"
  | "passChanged";

interface ActionError {
  reason: string;
  retryAfter?: number | null;
}

function errorOf(e: unknown): ActionError {
  if (isCommandError(e) && e.kind === "account")
    return { reason: e.reason, retryAfter: e.retryAfter };
  return { reason: "internal" };
}

/** The host part only, as the design shows it ("biblio.casa-ruiz.es", not the scheme). */
function hostOf(url: string): string {
  return url.replace(/^https?:\/\//, "").replace(/\/+$/, "");
}

const RESEND_SECONDS = 60;

/**
 * Sign in, create an account and recover a password (D77, hito 7.1), as one
 * overlay with several internal steps (Auth.dc.html). A dialog on desktop, a
 * full screen on mobile (ResponsiveDialog, the app's usual shape for this).
 */
export function AuthDialog({
  open,
  initialStep,
  googleError,
}: {
  open: boolean;
  initialStep?: "signin" | "signup" | "googleTotp";
  /** The way back from Google failed while this was not on screen: say why (D77). */
  googleError?: string;
}) {
  const { t, i18n } = useTranslation();
  const account = useAccount();

  const [step, setStep] = useState<Step>(
    initialStep === "googleTotp" ? "totp" : (initialStep ?? "signin"),
  );
  // The second factor being asked is the one of a way in with Google: the code goes to `finishGoogle`.
  const [viaGoogle, setViaGoogle] = useState(initialStep === "googleTotp");
  const [offersGoogle, setOffersGoogle] = useState(false);
  // Only some addresses may make an account here (D100): said before registering, without saying which.
  const [restricted, setRestricted] = useState(false);
  const [seeded, setSeeded] = useState(false);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [name, setName] = useState("");
  const [language, setLanguage] = useState<(typeof supportedLanguages)[number]>(
    (i18n.resolvedLanguage ?? "en") as (typeof supportedLanguages)[number],
  );
  const [serverUrl, setServerUrl] = useState("");
  const [serverInfo, setServerInfo] = useState<ServerInfo | null>(null);
  const [code, setCode] = useState("");
  const [newPassword, setNewPassword] = useState("");
  // The code of the second factor, asked to recover the password of an account that has it on (D100).
  const [appCode, setAppCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<ActionError | null>(null);
  const [resendAt, setResendAt] = useState(0);
  const [now, setNow] = useState(() => Date.now());

  // Fresh each time the overlay opens, prefilled from what the device already knows
  // (the same render-time reset PairDialog uses, instead of an effect).
  if (open && !seeded) {
    setSeeded(true);
    setStep(initialStep === "googleTotp" ? "totp" : (initialStep ?? "signin"));
    setViaGoogle(initialStep === "googleTotp");
    setEmail(account.emailHint ?? "");
    setPassword("");
    setName("");
    setServerUrl(account.official ? "" : account.server);
    setServerInfo(null);
    setCode("");
    setNewPassword("");
    setAppCode("");
    setError(googleError ? { reason: googleError } : null);
  } else if (!open && seeded) {
    setSeeded(false);
  }

  // Google is offered only if this server has it (D77: «the server says what it offers»).
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    accountServerInfo()
      .then((info) => {
        if (cancelled) return;
        setOffersGoogle(info.signInWithGoogle);
        setRestricted(info.registrationRestricted);
      })
      .catch(() => {
        if (cancelled) return;
        setOffersGoogle(false);
        setRestricted(false);
      });
    return () => {
      cancelled = true;
    };
  }, [open, account.server]);

  // The way back from Google, read by the app while this is open (`useArrivedLink`): it needs the second factor, or it
  // failed. A good one just signs in and closes this.
  useEffect(() => {
    if (!open) return;
    const onResult = (event: Event) => {
      const detail = (event as CustomEvent<{ reason: string }>).detail;
      setBusy(false);
      setCode("");
      if (detail.reason === "two_factor_required") {
        setViaGoogle(true);
        setStep("totp");
        setError(null);
      } else {
        setStep("signin");
        setError({ reason: detail.reason });
      }
    };
    window.addEventListener(GOOGLE_RESULT, onResult);
    return () => window.removeEventListener(GOOGLE_RESULT, onResult);
  }, [open]);

  useEffect(() => {
    if (resendAt === 0) return;
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [resendAt]);

  const resendLeft = resendAt
    ? Math.max(0, Math.ceil((resendAt + RESEND_SECONDS * 1000 - now) / 1000))
    : 0;

  const run = async (action: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(errorOf(e));
    } finally {
      setBusy(false);
    }
  };

  const afterSignedIn = async () => {
    await refreshAccount();
    closeOverlay();
  };

  const doGoogle = () =>
    run(async () => {
      await startGoogle(i18n.resolvedLanguage ?? "en");
      setViaGoogle(false);
      setStep("google");
    });

  const doGoogleCode = () =>
    run(async () => {
      try {
        await finishGoogle(code);
        await afterSignedIn();
      } catch (e) {
        // A wrong code leaves the way back waiting; anything else ends it: back to the start, saying why.
        const err = errorOf(e);
        if (err.reason === "invalid_two_factor_code") throw e;
        setViaGoogle(false);
        setStep("signin");
        throw e;
      }
    });

  const doSignIn = (totpCode: string | null) =>
    run(async () => {
      try {
        await signIn(email.trim(), password, totpCode);
        await afterSignedIn();
      } catch (e) {
        const err = errorOf(e);
        if (err.reason === "two_factor_required") {
          setStep("totp");
          setError(null);
          return;
        }
        throw e;
      }
    });

  const doRegister = () =>
    run(async () => {
      if (password.length < 8) {
        setError({ reason: "password_short" });
        return;
      }
      await registerAccount(email.trim(), password, name.trim(), language);
      setStep("verify");
      setResendAt(Date.now());
      setNow(Date.now());
    });

  const doVerify = () =>
    run(async () => {
      // The password is chosen here, with the code (D100): the one of whoever has the email.
      if (password.length < 8) {
        setError({ reason: "password_short" });
        return;
      }
      await confirmAccountEmail(email.trim(), code, password);
      // Creating an account ends signed in, with the password just typed.
      await signIn(email.trim(), password, null);
      await afterSignedIn();
    });

  const doResendVerify = () =>
    run(async () => {
      await resendAccountCode(email.trim());
      setResendAt(Date.now());
      setNow(Date.now());
    });

  /** An account that was never confirmed: a new code, and the screen to type it with the password it will have. */
  const doConfirmLater = () =>
    run(async () => {
      await resendAccountCode(email.trim());
      setResendAt(Date.now());
      setNow(Date.now());
      setCode("");
      setPassword("");
      setStep("verify");
    });

  const doForgot = () =>
    run(async () => {
      await forgotPassword(email.trim());
      setCode("");
      setStep("recoverCode");
    });

  const doRecoverCode = () => {
    if (code.length !== 6) return;
    setError(null);
    setStep("newPass");
  };

  const doNewPass = (totpCode: string | null) =>
    run(async () => {
      if (newPassword.length < 8) {
        setError({ reason: "password_short" });
        return;
      }
      try {
        await resetPassword(email.trim(), code, newPassword, totpCode);
        setStep("passChanged");
      } catch (e) {
        // The email alone is not enough for an account with the second factor on: its code, then the same request (D100).
        if (errorOf(e).reason === "two_factor_required") {
          setAppCode("");
          setStep("recoverTotp");
          return;
        }
        throw e;
      }
    });

  const doCheckServer = () =>
    run(async () => {
      const trimmed = serverUrl.trim();
      await setAccountServer(trimmed || null);
      const info = await accountServerInfo();
      setServerInfo(info);
      setOffersGoogle(info.signInWithGoogle);
      setRestricted(info.registrationRestricted);
    });

  const chooseOfficialServer = () =>
    run(async () => {
      await setAccountServer(null);
      setServerInfo(null);
      setServerUrl("");
      setStep("signin");
    });

  const close = () => {
    // Closing while the browser is open gives up that try: a way back that comes later is nobody's.
    if (step === "google" || viaGoogle) void cancelGoogle().catch(() => {});
    closeOverlay();
  };

  const host = account.official ? "kosmotheka.app" : hostOf(account.server);
  const resendTime = `${Math.floor(resendLeft / 60)}:${String(resendLeft % 60).padStart(2, "0")}`;

  const title = t(`account.auth.steps.${step === "server" ? "signin" : step}.title`);
  const primaryLabel = t(`account.auth.steps.${step === "server" ? "signin" : step}.primary`);
  const stepLabel = t(`account.auth.steps.${step}.stepLabel`, { defaultValue: "" });

  const field = (label: string, input: ReactNode) => (
    <label className="flex flex-col gap-1.5">
      <span className="text-sm font-medium">{label}</span>
      {input}
    </label>
  );

  const emailField = field(
    t("account.auth.email"),
    <Input
      type="email"
      autoComplete="email"
      value={email}
      onChange={(e) => setEmail(e.target.value)}
      disabled={busy}
    />,
  );

  const passwordField = (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-center justify-between text-sm font-medium">
        <label htmlFor="auth-password">{t("account.auth.password")}</label>
        {step !== "signup" && (
          <button
            type="button"
            className="font-medium text-primary"
            onClick={() => {
              setError(null);
              setStep("forgot");
            }}
          >
            {t("account.auth.forgotPassword")}
          </button>
        )}
      </div>
      <Input
        id="auth-password"
        type="password"
        autoComplete={step === "signup" ? "new-password" : "current-password"}
        value={password}
        onChange={(e) => setPassword(e.target.value)}
        disabled={busy}
      />
      {step === "signup" && (
        <span className="flex items-center gap-1.5 text-[13px] text-success">
          <Check className="size-3.5" /> {t("account.auth.passwordHint")}
        </span>
      )}
    </div>
  );

  const googleButton = offersGoogle && (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-3 text-[13px] text-muted-foreground">
        <span className="h-px flex-1 bg-border" />
        {t("account.auth.googleOr")}
        <span className="h-px flex-1 bg-border" />
      </div>
      <Button
        type="button"
        variant="outline"
        className="h-12"
        disabled={busy}
        onClick={() => void doGoogle()}
      >
        {t("account.auth.google")}
      </Button>
    </div>
  );

  let body: ReactNode;
  let footer: ReactNode = null;

  if (step === "signin" || step === "server") {
    body = (
      <>
        {step === "signin" ? (
          <div className="flex items-center gap-2 text-[13px] text-muted-foreground">
            <Globe className="size-[15px]" />
            <span>{t("account.auth.onServer", { host })}</span>
            <span aria-hidden="true">·</span>
            <button
              type="button"
              className="font-medium text-primary"
              onClick={() => setStep("server")}
            >
              {t("account.auth.otherServer")}
            </button>
          </div>
        ) : (
          <label className="flex flex-col gap-1.5">
            <span className="flex items-center justify-between text-sm font-medium">
              {t("account.auth.serverLabel")}
              <button
                type="button"
                className="font-medium text-primary"
                onClick={() => void chooseOfficialServer()}
              >
                {t("account.auth.useOfficial")}
              </button>
            </span>
            <div className="flex items-center gap-2 rounded-xl border border-ring px-3 ring-3 ring-ring/25">
              <Server className="size-[17px] shrink-0 text-muted-foreground" />
              <input
                className="h-11 flex-1 bg-transparent text-base outline-none"
                value={serverUrl}
                onChange={(e) => {
                  setServerUrl(e.target.value);
                  setServerInfo(null);
                }}
                onBlur={() => void doCheckServer()}
                disabled={busy}
                placeholder="https://…"
              />
            </div>
            {serverInfo && !error && (
              <span className="flex items-center gap-1.5 text-[13px] text-muted-foreground">
                <span className="size-1.5 rounded-full bg-success" />
                {t("account.auth.serverResponds")}
              </span>
            )}
          </label>
        )}
        {restricted && (
          <div
            role="status"
            className="flex flex-col gap-2 rounded-xl bg-muted p-3.5 text-sm text-pretty"
          >
            <span className="flex gap-3">
              <Info className="mt-0.5 size-[18px] shrink-0" />
              <span>{t("account.auth.byInvitation")}</span>
            </span>
            <button
              type="button"
              className="flex items-center gap-2 self-start pl-[30px] font-medium text-primary"
              onClick={() => openOverlay({ name: "invitation" })}
            >
              <MailOpen className="size-4" />
              {t("invitations.open.entry")}
            </button>
          </div>
        )}
        {emailField}
        {passwordField}
        {googleButton}
      </>
    );
    footer = (
      <span className="pt-1 text-center text-sm text-muted-foreground">
        {t("account.auth.footer.signin")}{" "}
        <button
          type="button"
          className="font-semibold text-primary"
          onClick={() => setStep("signup")}
        >
          {t("account.auth.footer.signinLink")}
        </button>
      </span>
    );
  } else if (step === "signup") {
    body = (
      <>
        {field(
          t("account.auth.name"),
          <>
            <Input value={name} onChange={(e) => setName(e.target.value)} disabled={busy} />
            <span className="text-[13px] text-muted-foreground">{t("account.auth.nameHint")}</span>
          </>,
        )}
        {emailField}
        {passwordField}
        {restricted && (
          <div role="status" className="flex gap-3 rounded-xl bg-muted p-3.5 text-sm text-pretty">
            <Info className="mt-0.5 size-[18px] shrink-0" />
            <span>{t("account.auth.restrictedServer")}</span>
          </div>
        )}
        {field(
          t("account.auth.language"),
          <div className="flex gap-2">
            {supportedLanguages.map((lang) => (
              <Button
                key={lang}
                type="button"
                size="sm"
                variant={language === lang ? "secondary" : "outline"}
                onClick={() => setLanguage(lang)}
              >
                {lang === "en" ? "English" : "Español"}
              </Button>
            ))}
          </div>,
        )}
        {googleButton}
      </>
    );
    footer = (
      <span className="pt-1 text-center text-sm text-muted-foreground">
        {t("account.auth.footer.signup")}{" "}
        <button
          type="button"
          className="font-semibold text-primary"
          onClick={() => setStep("signin")}
        >
          {t("account.auth.footer.signupLink")}
        </button>
      </span>
    );
  } else if (step === "verify" || step === "recoverCode") {
    const resend = step === "verify" ? doResendVerify : doForgot;
    body = (
      <>
        <CodeInput value={code} onChange={setCode} disabled={busy} invalid={Boolean(error)} />
        {step === "verify" && password.length < 8 && (
          <>
            {field(
              t("account.auth.verifyPassword"),
              <>
                <Input
                  type="password"
                  autoComplete="new-password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  disabled={busy}
                />
                <span className="flex items-center gap-1.5 text-[13px] text-success">
                  <Check className="size-3.5" /> {t("account.auth.passwordHint")}
                </span>
              </>,
            )}
          </>
        )}
        <div className="flex flex-wrap gap-x-5 gap-y-2 text-sm">
          {resendLeft > 0 ? (
            <span className="text-muted-foreground">
              {t("account.auth.resendIn", { time: resendTime })}
            </span>
          ) : (
            <button
              type="button"
              className="font-medium text-primary"
              onClick={() => void resend()}
            >
              {t("account.auth.resend")}
            </button>
          )}
          <button
            type="button"
            className="font-medium text-primary"
            onClick={() => {
              setError(null);
              setStep(step === "verify" ? "signup" : "forgot");
            }}
          >
            {t("account.auth.changeEmail")}
          </button>
        </div>
      </>
    );
  } else if (step === "google") {
    body = (
      <p role="status" className="text-sm text-muted-foreground">
        {t("account.auth.googleWaiting")}
      </p>
    );
  } else if (step === "totp" || step === "backup") {
    body =
      step === "totp" ? (
        <>
          <CodeInput value={code} onChange={setCode} disabled={busy} invalid={Boolean(error)} />
          <button
            type="button"
            className="self-start text-sm font-medium text-primary"
            onClick={() => {
              setError(null);
              setCode("");
              setStep("backup");
            }}
          >
            {t("account.auth.useBackupCode")}
          </button>
        </>
      ) : (
        <>
          {field(
            t("account.auth.backupCodeLabel"),
            <>
              <Input
                value={code}
                onChange={(e) => setCode(e.target.value)}
                disabled={busy}
                className="font-mono"
              />
              <span className="text-[13px] text-muted-foreground">
                {t("account.auth.backupCodeHint")}
              </span>
            </>,
          )}
          <button
            type="button"
            className="self-start text-sm font-medium text-primary"
            onClick={() => {
              setError(null);
              setCode("");
              setStep("totp");
            }}
          >
            {t("account.auth.useAuthApp")}
          </button>
        </>
      );
  } else if (step === "recoverTotp") {
    body = (
      <SecondFactorInput
        value={appCode}
        onChange={setAppCode}
        disabled={busy}
        invalid={Boolean(error)}
      />
    );
  } else if (step === "forgot") {
    body = (
      <>
        {emailField}
        <div className="flex gap-3 rounded-xl bg-muted p-3.5 text-sm text-pretty">
          <LogOut className="mt-0.5 size-[18px] shrink-0" />
          <span>{t("account.auth.otherDevicesWarning")}</span>
        </div>
      </>
    );
  } else if (step === "newPass") {
    body = (
      <>
        {field(
          t("account.auth.newPasswordLabel"),
          <>
            <Input
              type="password"
              autoComplete="new-password"
              value={newPassword}
              onChange={(e) => setNewPassword(e.target.value)}
              disabled={busy}
            />
            <span className="flex items-center gap-1.5 text-[13px] text-success">
              <Check className="size-3.5" /> {t("account.auth.passwordHint")}
            </span>
          </>,
        )}
        <div className="flex gap-3 rounded-xl bg-muted p-3.5 text-sm text-pretty">
          <LogOut className="mt-0.5 size-[18px] shrink-0" />
          <span>{t("account.auth.otherDevicesWarning")}</span>
        </div>
      </>
    );
  } else {
    // passChanged
    body = null;
  }

  const primaryAction = () => {
    if (step === "signin" || step === "server") void doSignIn(null);
    else if (step === "signup") void doRegister();
    else if (step === "verify") void doVerify();
    else if (step === "google") {
      // «Cancel»: the way back that may still come is no longer this app's.
      void cancelGoogle().catch(() => {});
      setStep("signin");
    } else if (step === "totp" || step === "backup") {
      if (viaGoogle) void doGoogleCode();
      else void doSignIn(code);
    } else if (step === "forgot") void doForgot();
    else if (step === "recoverCode") doRecoverCode();
    else if (step === "newPass") void doNewPass(null);
    else if (step === "recoverTotp") void doNewPass(appCode);
    else {
      // "passChanged", the last step: back to signing in.
      setStep("signin");
      setPassword("");
      setError(null);
    }
  };

  const primaryDisabled =
    busy ||
    ((step === "signin" || step === "server") && (!email.trim() || !password)) ||
    (step === "signup" && (!name.trim() || !email.trim() || password.length < 8)) ||
    ((step === "verify" || step === "recoverCode" || step === "totp") && code.length !== 6) ||
    (step === "verify" && password.length < 8) ||
    (step === "recoverTotp" && !isSecondFactorCode(appCode)) ||
    (step === "backup" && !code.trim()) ||
    (step === "forgot" && !email.trim()) ||
    (step === "newPass" && newPassword.length < 8);

  return (
    <ResponsiveDialog
      open={open}
      onOpenChange={(next) => !next && close()}
      title={title}
      description={stepLabel || undefined}
    >
      <div className="flex flex-col gap-5">
        {step === "passChanged" && (
          <span className="flex size-14 items-center justify-center rounded-full bg-success-soft text-success">
            <Check className="size-[26px]" />
          </span>
        )}
        <p className="text-[15px] text-pretty text-muted-foreground">
          {t(`account.auth.steps.${step}.sub`, { email, host })}
        </p>
        {body}
        {error && (
          <AuthErrorBlock
            reason={error.reason}
            retryAfter={error.retryAfter}
            totpStep={step === "totp" || step === "recoverTotp"}
            onAction={() => {
              if (error.reason === "invalid_credentials") setStep("forgot");
              else if (error.reason === "account_not_confirmed") void doConfirmLater();
              else if (error.reason === "code_expired" || error.reason === "too_many_attempts") {
                void (step === "recoverCode" ? doForgot() : doResendVerify());
              } else if (error.reason === "invalid_two_factor_code" && step === "totp") {
                setCode("");
                setStep("backup");
              } else if (error.reason === "server_unavailable") {
                if (step === "server") void doCheckServer();
                else primaryAction();
              }
            }}
          />
        )}
        <Button className="h-12" disabled={primaryDisabled} onClick={primaryAction}>
          {primaryLabel}
        </Button>
        {footer}
      </div>
    </ResponsiveDialog>
  );
}
