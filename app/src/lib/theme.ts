// Light or dark: the system's by default, or forced from Settings. Kept in
// localStorage, like the language, so it applies before the first render.

export const themeChoices = ["system", "light", "dark"] as const;
export type ThemeChoice = (typeof themeChoices)[number];

const STORAGE_KEY = "kosmotheka.theme";
const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

declare global {
  interface Window {
    /** Added by the Android app (MainActivity.kt) to colour the system bar icons. */
    KosmothekaSystemBars?: { setDark(dark: boolean): void };
  }
}

export function themeChoice(): ThemeChoice {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return themeChoices.find((choice) => choice === stored) ?? "system";
  } catch {
    return "system";
  }
}

export function setThemeChoice(choice: ThemeChoice) {
  try {
    if (choice === "system") localStorage.removeItem(STORAGE_KEY);
    else localStorage.setItem(STORAGE_KEY, choice);
  } catch {
    // Without storage the choice still applies until the app closes.
  }
  applyTheme(choice);
}

/** shadcn/ui switches themes with a `dark` class on the root element. */
function applyTheme(choice = themeChoice()) {
  const dark = choice === "dark" || (choice === "system" && darkQuery.matches);
  document.documentElement.classList.toggle("dark", dark);
  window.KosmothekaSystemBars?.setDark(dark);
}

export function initTheme() {
  applyTheme();
  darkQuery.addEventListener("change", () => applyTheme());
}
