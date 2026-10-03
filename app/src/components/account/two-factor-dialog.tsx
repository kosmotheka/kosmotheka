import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import QRCode from "qrcode";
import { Copy, ShieldCheck } from "lucide-react";

import { AuthErrorBlock } from "@/components/account/auth-error-block";
import { CodeInput } from "@/components/account/code-input";
import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { type AccountActionError, accountErrorOf } from "@/lib/account-errors";
import { type TwoFactorSetup, twoFactorEnable, twoFactorSetup } from "@/lib/api";
import { closeOverlay, openOverlay } from "@/lib/navigation";

/** The secret in groups of 4, as the design shows it ("JBSW Y3DP EHPK 3PXP"). */
function grouped(secret: string): string {
  return secret.replace(/(.{4})/g, "$1 ").trim();
}

type Step = "setup" | "codes";

/**
 * Turning on the second factor, or regenerating backup codes (TwoFactor.dc.html):
 * QR and secret, the first code from the app, then the ten backup codes, shown
 * only this once.
 */
export function TwoFactorDialog({
  open,
  desktop,
  codes: givenCodes,
}: {
  open: boolean;
  desktop: boolean;
  /** Set when opened from "Códigos nuevos": skip setup, show these directly. */
  codes?: string[];
}) {
  const { t } = useTranslation();
  const [step, setStep] = useState<Step>(givenCodes ? "codes" : "setup");
  const [setup, setSetup] = useState<TwoFactorSetup | null>(null);
  const [qr, setQr] = useState<string | null>(null);
  const [code, setCode] = useState("");
  const [codes, setCodes] = useState<string[]>(givenCodes ?? []);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AccountActionError | null>(null);
  const [seeded, setSeeded] = useState(false);
  const [askKeepCodes, setAskKeepCodes] = useState(false);

  if (open && !seeded) {
    setSeeded(true);
    setStep(givenCodes ? "codes" : "setup");
    setCodes(givenCodes ?? []);
    setCode("");
    setError(null);
    setAskKeepCodes(false);
  } else if (!open && seeded) {
    setSeeded(false);
  }

  useEffect(() => {
    if (!open || step !== "setup") return;
    let cancelled = false;
    twoFactorSetup()
      .then(async (loaded) => {
        if (cancelled) return;
        setSetup(loaded);
        setQr(await QRCode.toString(loaded.otpauthUri, { type: "svg", margin: 1 }));
      })
      .catch((e: unknown) => !cancelled && setError(accountErrorOf(e)));
    return () => {
      cancelled = true;
    };
  }, [open, step]);

  const enable = async () => {
    setBusy(true);
    setError(null);
    try {
      const generated = await twoFactorEnable(code);
      setCodes(generated);
      setStep("codes");
    } catch (e) {
      setError(accountErrorOf(e));
    } finally {
      setBusy(false);
    }
  };

  const copySecret = () => {
    if (setup) void navigator.clipboard.writeText(setup.secret).catch(() => {});
  };

  const copyCodes = () => {
    void navigator.clipboard.writeText(codes.join("\n")).catch(() => {});
  };

  /** "Ya los he guardado", and any other way of leaving step 2 once confirmed. */
  const backToAccount = () => openOverlay({ name: "account" });

  const requestClose = () => {
    if (step === "codes") setAskKeepCodes(true);
    else closeOverlay();
  };

  return (
    <>
      <ResponsiveDialog
        open={open}
        onOpenChange={(next) => !next && requestClose()}
        title={t(
          step === "setup" ? "account.twoFactorSetup.title" : "account.twoFactorCodes.title",
        )}
        description={t(
          step === "setup"
            ? "account.twoFactorSetup.stepLabel1"
            : "account.twoFactorSetup.stepLabel2",
        )}
      >
        {step === "setup" ? (
          <div className="flex flex-col gap-5">
            <p className="text-[15px] text-pretty text-muted-foreground">
              {t("account.twoFactorSetup.sub")}
            </p>
            <div className="flex items-center gap-4">
              {qr ? (
                <div
                  className="w-[132px] shrink-0 rounded-xl border bg-white p-2"
                  dangerouslySetInnerHTML={{ __html: qr }}
                />
              ) : (
                <div className="size-[132px] shrink-0 animate-pulse rounded-xl bg-muted" />
              )}
              <div className="flex min-w-0 flex-1 flex-col gap-2">
                <span className="text-sm font-medium">{t("account.twoFactorSetup.step1")}</span>
                <span className="text-[13px] text-pretty text-muted-foreground">
                  {t("account.twoFactorSetup.cantScan")}
                </span>
                {setup && (
                  <div className="flex items-center gap-1.5 rounded-lg bg-muted py-1.5 pl-2.5 pr-1.5">
                    <span className="flex-1 font-mono text-[13px] tracking-wide break-all">
                      {grouped(setup.secret)}
                    </span>
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      aria-label={t("account.twoFactorSetup.copySecret")}
                      onClick={copySecret}
                    >
                      <Copy className="size-4" />
                    </Button>
                  </div>
                )}
              </div>
            </div>
            <div className="flex flex-col gap-2.5">
              <span className="text-sm font-medium">{t("account.twoFactorSetup.step2")}</span>
              <CodeInput value={code} onChange={setCode} disabled={busy} invalid={Boolean(error)} />
            </div>
            {error && <AuthErrorBlock reason={error.reason} retryAfter={error.retryAfter} />}
            <Button
              className="h-12"
              disabled={busy || code.length !== 6}
              onClick={() => void enable()}
            >
              {t("account.twoFactorSetup.enable")}
            </Button>
          </div>
        ) : (
          <div className="flex flex-col gap-5">
            <span className="flex items-center gap-2 text-sm font-medium text-success">
              <ShieldCheck className="size-[17px]" /> {t("account.twoFactorCodes.enabled")}
            </span>
            <p className="text-[15px] text-pretty text-muted-foreground">
              {t("account.twoFactorCodes.sub")}{" "}
              <strong className="font-semibold text-foreground">
                {t("account.twoFactorCodes.onceOnly")}
              </strong>
            </p>
            <div className="grid grid-cols-2 gap-x-4 gap-y-2 rounded-2xl bg-muted px-5 py-4 font-mono text-[15px] tracking-wide">
              {codes.map((c) => (
                <span key={c}>{c}</span>
              ))}
            </div>
            <Button variant="outline" className="h-11" onClick={copyCodes}>
              <Copy /> {t("account.twoFactorCodes.copy")}
            </Button>
            <Button className="h-12" onClick={backToAccount}>
              {t("account.twoFactorCodes.done")}
            </Button>
          </div>
        )}
      </ResponsiveDialog>

      <SheetOrDialog
        open={askKeepCodes}
        desktop={desktop}
        onClose={() => setAskKeepCodes(false)}
        title={t("account.twoFactorCodes.closeConfirm.title")}
        description={t("account.twoFactorCodes.closeConfirm.title")}
      >
        <div className="flex flex-col gap-4">
          <p className="text-[15px] text-pretty text-muted-foreground">
            {t("account.twoFactorCodes.closeConfirm.body")}
          </p>
          <div className="grid grid-cols-2 gap-2.5">
            <Button variant="outline" className="h-11" onClick={() => setAskKeepCodes(false)}>
              {t("account.twoFactorCodes.closeConfirm.back")}
            </Button>
            <Button
              variant="destructive"
              className="h-11"
              onClick={() => {
                setAskKeepCodes(false);
                backToAccount();
              }}
            >
              {t("account.twoFactorCodes.closeConfirm.confirm")}
            </Button>
          </div>
        </div>
      </SheetOrDialog>
    </>
  );
}
