import { type Language, languageOf } from "@/i18n";

/** The downloads page in each language that has one; the others get the fallback language's (D104). */
const DOWNLOADS: Partial<Record<Language, string>> & { en: string } = {
  es: "https://kosmotheka.com/es/descargas.html",
  en: "https://kosmotheka.com/en/download.html",
};

export function downloadsPage(language: string): string {
  return DOWNLOADS[languageOf(language)] ?? DOWNLOADS.en;
}
