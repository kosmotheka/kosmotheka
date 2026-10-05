// A work's language is stored as an ISO 639-1 code ("es", "en"), as the providers
// return it, and shown by name in the app's own language through Intl.

/** The languages offered in the form. Any other stored code is still kept and shown. */
export const LANGUAGE_CODES = [
  "es",
  "ca",
  "gl",
  "eu",
  "en",
  "fr",
  "de",
  "it",
  "pt",
  "nl",
  "sv",
  "da",
  "no",
  "fi",
  "is",
  "pl",
  "cs",
  "sk",
  "hu",
  "ro",
  "bg",
  "el",
  "ru",
  "uk",
  "tr",
  "ar",
  "he",
  "fa",
  "hi",
  "bn",
  "ur",
  "id",
  "ms",
  "vi",
  "th",
  "zh",
  "ja",
  "ko",
  "sw",
  "la",
  "eo",
] as const;

/** "es" → "Español" in Spanish, "Spanish" in English; unknown text comes back as it is. */
export function languageName(code: string, uiLanguage: string): string {
  try {
    const name = new Intl.DisplayNames([uiLanguage], { type: "language" }).of(code);
    if (name && name !== code) return name.charAt(0).toLocaleUpperCase(uiLanguage) + name.slice(1);
  } catch {
    // Not a valid code (free text from an older version): shown as written.
  }
  return code;
}
