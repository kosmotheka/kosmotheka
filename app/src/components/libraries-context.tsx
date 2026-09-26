import { createContext, useContext } from "react";
import { useTranslation } from "react-i18next";

import type { LibrarySummary } from "@/lib/api";

export interface Libraries {
  all: LibrarySummary[];
  active: LibrarySummary | null;
  /** Makes another library the active one and reloads what depends on it. */
  switchTo: (id: string) => Promise<void>;
  reload: () => void;
}

export const LibrariesContext = createContext<Libraries>({
  all: [],
  active: null,
  switchTo: async () => {},
  reload: () => {},
});

export function useLibraries() {
  return useContext(LibrariesContext);
}

/** A library never named shows the app's default name, in the user's language. */
export function useLibraryName() {
  const { t } = useTranslation();
  return (library: Pick<LibrarySummary, "name"> | null | undefined) =>
    library?.name || t("nav.myLibrary");
}
