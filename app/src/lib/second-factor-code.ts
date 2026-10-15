// The alphabet of server/src/Security/TwoFactor.php (BACKUP_CODE_ALPHABET): no 0, O, 1, I or L, which are easy to
// misread off a screen or a printed card.
const BACKUP_CODE = /^[23456789A-HJKMNP-Z]{5}-?[23456789A-HJKMNP-Z]{5}$/;

/**
 * Whether `raw` can be a code of the second factor (D78, D100): the 6 digits of the authenticator app, or one of the
 * backup codes (XXXXX-XXXXX), with or without the dash, in capitals or not. The server normalises the same way.
 */
export function isSecondFactorCode(raw: string): boolean {
  const code = raw.trim();
  return /^\d{6}$/.test(code) || BACKUP_CODE.test(code.toUpperCase());
}
