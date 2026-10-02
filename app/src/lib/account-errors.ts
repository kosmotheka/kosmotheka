// Maps a server account error reason to its copy and severity (docs/design/errors.md,
// "Cuenta y nube (Fase 7)"). One place for the mapping, so every auth screen shows the
// same block instead of each re-deciding the color and wording.

export type AccountErrorLevel = "error" | "warning" | "info";

export interface AccountErrorText {
  level: AccountErrorLevel;
  titleKey: string;
  /** Omitted when the design has no body text under the title (e.g. `password_short`). */
  helpKey?: string;
  /** Translation key for the action link's label, when the error offers one. */
  actionKey?: string;
}

/** Minutes to show for `too_many_requests`, rounded up, at least 1. */
export function retryAfterMinutes(retryAfterSeconds: number | null | undefined): number {
  return Math.max(1, Math.ceil((retryAfterSeconds ?? 0) / 60));
}

/**
 * `totpStep` picks between the two `invalid_two_factor_code` texts: entering a TOTP
 * code or a backup code. Returns `null` for anything not in the design, which the
 * caller falls back to the generic internal error for.
 */
export function accountErrorText(reason: string, totpStep = true): AccountErrorText | null {
  switch (reason) {
    case "invalid_code":
      return {
        level: "error",
        titleKey: "accountErrors.invalidCode.title",
        helpKey: "accountErrors.invalidCode.help",
      };
    case "code_expired":
      return {
        level: "error",
        titleKey: "accountErrors.codeExpired.title",
        helpKey: "accountErrors.codeExpired.help",
        actionKey: "accountErrors.codeExpired.action",
      };
    case "too_many_attempts":
      return {
        level: "error",
        titleKey: "accountErrors.tooManyAttempts.title",
        helpKey: "accountErrors.tooManyAttempts.help",
        actionKey: "accountErrors.tooManyAttempts.action",
      };
    case "too_many_requests":
      return {
        level: "warning",
        titleKey: "accountErrors.tooManyRequests.title",
        helpKey: "accountErrors.tooManyRequests.help",
      };
    case "invalid_credentials":
      return {
        level: "error",
        titleKey: "accountErrors.invalidCredentials.title",
        helpKey: "accountErrors.invalidCredentials.help",
        actionKey: "accountErrors.invalidCredentials.action",
      };
    case "account_not_confirmed":
      return {
        level: "warning",
        titleKey: "accountErrors.accountNotConfirmed.title",
        helpKey: "accountErrors.accountNotConfirmed.help",
        actionKey: "accountErrors.accountNotConfirmed.action",
      };
    case "account_blocked":
      return {
        level: "error",
        titleKey: "accountErrors.accountBlocked.title",
        helpKey: "accountErrors.accountBlocked.help",
      };
    case "invalid_two_factor_code":
      return totpStep
        ? {
            level: "error",
            titleKey: "accountErrors.invalidTotp.title",
            helpKey: "accountErrors.invalidTotp.help",
            actionKey: "accountErrors.invalidTotp.action",
          }
        : {
            level: "error",
            titleKey: "accountErrors.invalidBackupCode.title",
            helpKey: "accountErrors.invalidBackupCode.help",
          };
    case "server_unavailable":
      return {
        level: "warning",
        titleKey: "accountErrors.serverUnavailable.title",
        helpKey: "accountErrors.serverUnavailable.help",
        actionKey: "accountErrors.serverUnavailable.action",
      };
    case "password_short":
      return { level: "error", titleKey: "accountErrors.passwordShort.title" };
    case "password_long":
      return {
        level: "error",
        titleKey: "accountErrors.passwordLong.title",
        helpKey: "accountErrors.passwordLong.help",
      };
    case "invalid_input":
      return {
        level: "error",
        titleKey: "accountErrors.invalidInput.title",
        helpKey: "accountErrors.invalidInput.help",
      };
    case "invalid_server":
      return {
        level: "error",
        titleKey: "accountErrors.invalidServer.title",
        helpKey: "accountErrors.invalidServer.help",
      };
    case "device_removed":
      return { level: "info", titleKey: "accountErrors.deviceRemoved.title" };
    case "signed_out_password_changed":
      return {
        level: "info",
        titleKey: "accountErrors.signedOut.title",
        helpKey: "accountErrors.signedOut.help",
      };
    default:
      return null;
  }
}
