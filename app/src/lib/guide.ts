import { type Language, languageOf } from "@/i18n";

/** The guide's «Using the app» part in each language that has one; the others get the fallback language's. */
const GUIDE: Partial<Record<Language, string>> & { en: string } = {
  es: "https://docs.kosmotheka.com/es/using-the-app/",
  en: "https://docs.kosmotheka.com/en/using-the-app/",
};

/** A page of the guide, by its path, the same in every language (`search-better/`); the guide itself without one. */
export function guidePage(language: string, page = ""): string {
  return (GUIDE[languageOf(language)] ?? GUIDE.en) + page;
}
