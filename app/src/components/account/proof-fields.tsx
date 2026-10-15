import { useState } from "react";
import { useTranslation } from "react-i18next";

import { AuthErrorBlock } from "@/components/account/auth-error-block";
import { CodeInput } from "@/components/account/code-input";
import { SecondFactorInput } from "@/components/account/second-factor-input";
import { Field } from "@/components/book-form";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { type AccountActionError, accountErrorOf } from "@/lib/account-errors";
import { isSecondFactorCode } from "@/lib/second-factor-code";
import { type ActionProof, requestActionCode } from "@/lib/api";

/** What the person has typed to confirm a delicate action (D100). */
export interface ProofState {
  password: string;
  emailCode: string;
  totpCode: string;
}

export const emptyProof: ProofState = { password: "", emailCode: "", totpCode: "" };

/**
 * What an account is asked, besides its session, before something that changes how it is entered (D100): who is
 * asking, by the password or, for an account with none (it came in with Google), a code sent to its email; and the
 * code of the second factor, if it has it on. Phone lock is not asked.
 */
export interface ProofNeeds {
  who: "password" | "email" | "none";
  secondFactor: boolean;
}

/** Everything the account must give is there. */
export function proofComplete(proof: ProofState, needs: ProofNeeds): boolean {
  const who =
    needs.who === "password"
      ? proof.password !== ""
      : needs.who === "email"
        ? proof.emailCode.length === 6
        : true;
  return who && (!needs.secondFactor || isSecondFactorCode(proof.totpCode));
}

/** What goes to the server: only what the account is asked for. */
export function toActionProof(proof: ProofState, needs: ProofNeeds): ActionProof {
  return {
    ...(needs.who === "password" ? { password: proof.password } : {}),
    ...(needs.who === "email" ? { emailCode: proof.emailCode } : {}),
    ...(needs.secondFactor ? { totpCode: proof.totpCode } : {}),
  };
}

/** The fields for `ProofNeeds`: the password, or the way to ask for the email code, and the app's code. */
export function ProofFields({
  idPrefix,
  needs,
  proof,
  onChange,
  disabled,
  invalid,
}: {
  idPrefix: string;
  needs: ProofNeeds;
  proof: ProofState;
  onChange: (proof: ProofState) => void;
  disabled?: boolean;
  invalid?: boolean;
}) {
  const { t } = useTranslation();
  const [sent, setSent] = useState(false);
  const [sending, setSending] = useState(false);
  const [sendError, setSendError] = useState<AccountActionError | null>(null);

  const sendCode = async () => {
    setSending(true);
    setSendError(null);
    try {
      await requestActionCode();
      setSent(true);
    } catch (e) {
      setSendError(accountErrorOf(e));
    } finally {
      setSending(false);
    }
  };

  return (
    <>
      {needs.who === "password" && (
        <Field id={`${idPrefix}-password`} label={t("account.confirm.passwordLabel")}>
          <Input
            id={`${idPrefix}-password`}
            type="password"
            autoComplete="current-password"
            value={proof.password}
            onChange={(e) => onChange({ ...proof, password: e.target.value })}
            disabled={disabled}
          />
        </Field>
      )}
      {needs.who === "email" && (
        <div className="flex flex-col gap-2.5">
          <p className="text-[13px] text-pretty text-muted-foreground">
            {t("account.proof.noPassword")}
          </p>
          {sent ? (
            <>
              <Field id={`${idPrefix}-email-code`} label={t("account.proof.emailCodeLabel")}>
                <CodeInput
                  value={proof.emailCode}
                  onChange={(emailCode) => onChange({ ...proof, emailCode })}
                  disabled={disabled}
                  invalid={invalid}
                />
              </Field>
              <button
                type="button"
                className="self-start text-sm font-medium text-primary"
                disabled={sending || disabled}
                onClick={() => void sendCode()}
              >
                {t("account.proof.sendAgain")}
              </button>
            </>
          ) : (
            <Button
              type="button"
              variant="outline"
              className="h-11"
              disabled={sending || disabled}
              onClick={() => void sendCode()}
            >
              {t("account.proof.sendCode")}
            </Button>
          )}
          {sendError && (
            <AuthErrorBlock reason={sendError.reason} retryAfter={sendError.retryAfter} />
          )}
        </div>
      )}
      {needs.secondFactor && (
        <Field id={`${idPrefix}-app-code`} label={t("account.confirm.appCodeLabel")}>
          <SecondFactorInput
            id={`${idPrefix}-app-code`}
            value={proof.totpCode}
            onChange={(totpCode) => onChange({ ...proof, totpCode })}
            disabled={disabled}
            invalid={invalid}
          />
        </Field>
      )}
    </>
  );
}
