import { isCommandError } from "@/lib/api";

// Maps a server account error reason to its copy and severity (docs/design/errors.md,
// "Cuenta y nube (Fase 7)"). One place for the mapping, so every auth screen shows the
// same block instead of each re-deciding the color and wording.

type AccountErrorLevel = "error" | "warning" | "info";

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

export interface AccountActionError {
  reason: string;
  retryAfter?: number | null;
}

/** The account screens (account, second factor, devices) all read a thrown
 * command error the same way AuthDialog does. */
export function accountErrorOf(e: unknown): AccountActionError {
  if (isCommandError(e) && e.kind === "account")
    return { reason: e.reason, retryAfter: e.retryAfter };
  return { reason: "internal" };
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
    case "registration_closed":
      return {
        level: "warning",
        titleKey: "accountErrors.registrationClosed.title",
        helpKey: "accountErrors.registrationClosed.help",
      };
    case "google_cancelled":
      return { level: "info", titleKey: "accountErrors.googleCancelled.title" };
    case "google_expired":
    case "invalid_google_code":
      return {
        level: "warning",
        titleKey: "accountErrors.googleExpired.title",
        helpKey: "accountErrors.googleExpired.help",
      };
    case "google_email_not_verified":
      return {
        level: "error",
        titleKey: "accountErrors.googleEmailNotVerified.title",
        helpKey: "accountErrors.googleEmailNotVerified.help",
      };
    case "google_other_account":
      return {
        level: "error",
        titleKey: "accountErrors.googleOtherAccount.title",
        helpKey: "accountErrors.googleOtherAccount.help",
      };
    case "google_failed":
    case "google_unavailable":
      return {
        level: "error",
        titleKey: "accountErrors.googleFailed.title",
        helpKey: "accountErrors.googleFailed.help",
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
    case "device_taken":
      return {
        level: "error",
        titleKey: "accountErrors.deviceTaken.title",
        helpKey: "accountErrors.deviceTaken.help",
      };
    case "device_key_mismatch":
      return {
        level: "error",
        titleKey: "accountErrors.deviceKeyMismatch.title",
        helpKey: "accountErrors.deviceKeyMismatch.help",
      };
    case "signed_out_password_changed":
      return {
        level: "info",
        titleKey: "accountErrors.signedOut.title",
        helpKey: "accountErrors.signedOut.help",
      };
    case "two_factor_unavailable":
      return {
        level: "error",
        titleKey: "accountErrors.twoFactorUnavailable.title",
        helpKey: "accountErrors.twoFactorUnavailable.help",
      };
    case "two_factor_already_enabled":
      return {
        level: "warning",
        titleKey: "accountErrors.twoFactorAlreadyEnabled.title",
        helpKey: "accountErrors.twoFactorAlreadyEnabled.help",
      };
    case "two_factor_not_started":
      return {
        level: "error",
        titleKey: "accountErrors.twoFactorNotStarted.title",
        helpKey: "accountErrors.twoFactorNotStarted.help",
      };
    case "two_factor_not_enabled":
      return {
        level: "warning",
        titleKey: "accountErrors.twoFactorNotEnabled.title",
        helpKey: "accountErrors.twoFactorNotEnabled.help",
      };
    case "device_not_found":
      return { level: "info", titleKey: "accountErrors.deviceNotFound.title" };
    case "device_already_reset":
      return {
        level: "warning",
        titleKey: "accountErrors.deviceAlreadyReset.title",
        helpKey: "accountErrors.deviceAlreadyReset.help",
      };
    case "no_password":
      return {
        level: "error",
        titleKey: "accountErrors.noPassword.title",
        helpKey: "accountErrors.noPassword.help",
      };
    case "not_signed_in":
      return {
        level: "info",
        titleKey: "accountErrors.notSignedIn.title",
        helpKey: "accountErrors.notSignedIn.help",
        actionKey: "accountErrors.notSignedIn.action",
      };
    case "not_found":
      return {
        level: "error",
        titleKey: "accountErrors.notFound.title",
        helpKey: "accountErrors.notFound.help",
      };
    case "invalid_request":
      return {
        level: "error",
        titleKey: "accountErrors.invalidRequest.title",
        helpKey: "accountErrors.invalidRequest.help",
      };
    case "schema_version":
      return {
        level: "warning",
        titleKey: "accountErrors.schemaVersion.title",
        helpKey: "accountErrors.schemaVersion.help",
      };
    case "link_rejected":
      return {
        level: "error",
        titleKey: "accountErrors.linkRejected.title",
        helpKey: "accountErrors.linkRejected.help",
      };
    case "link_not_allowed":
      return {
        level: "info",
        titleKey: "accountErrors.linkNotAllowed.title",
        helpKey: "accountErrors.linkNotAllowed.help",
      };
    case "events_unavailable":
      return {
        level: "info",
        titleKey: "accountErrors.eventsUnavailable.title",
        helpKey: "accountErrors.eventsUnavailable.help",
      };
    case "access_withdrawn":
      return {
        level: "warning",
        titleKey: "accountErrors.accessWithdrawn.title",
        helpKey: "accountErrors.accessWithdrawn.help",
      };
    default:
      return null;
  }
}
