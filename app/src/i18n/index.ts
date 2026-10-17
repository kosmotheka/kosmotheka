import i18n from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import { initReactI18next } from "react-i18next";

import en from "./locales/en.json";
import es from "./locales/es.json";

/**
 * The one list of interface languages: add the code here, its file in `locales/` and its name below. Everything else
 * (the pickers, the news, the privacy link, the account language) reads this.
 */
export const supportedLanguages = ["en", "es"] as const;
export type Language = (typeof supportedLanguages)[number];

/** What a language is called, in itself, so it can be found whatever the current one is. */
export const languageNames: Record<Language, string> = {
  en: "English",
  es: "Español",
};

/** The language used when the person's has no text for something. */
const fallbackLanguage: Language = "en";

/** The supported language a browser or account language (`es`, `es-ES`) belongs to, or the fallback. */
export function languageOf(code: string | null | undefined): Language {
  return supportedLanguages.find((language) => code?.startsWith(language)) ?? fallbackLanguage;
}

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      en: { translation: en },
      es: { translation: es },
    },
    supportedLngs: supportedLanguages,
    fallbackLng: fallbackLanguage,
    nonExplicitSupportedLngs: true,
    interpolation: {
      // React already escapes rendered values.
      escapeValue: false,
    },
  });

export default i18n;
