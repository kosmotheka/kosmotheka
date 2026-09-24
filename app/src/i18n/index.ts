import i18n from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import { initReactI18next } from "react-i18next";

import en from "./locales/en.json";
import es from "./locales/es.json";

export const supportedLanguages = ["en", "es"] as const;

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      en: { translation: en },
      es: { translation: es },
    },
    supportedLngs: supportedLanguages,
    fallbackLng: "en",
    nonExplicitSupportedLngs: true,
    interpolation: {
      // React already escapes rendered values.
      escapeValue: false,
    },
  });

export default i18n;
