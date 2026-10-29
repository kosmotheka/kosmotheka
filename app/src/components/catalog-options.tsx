import { createContext, useContext } from "react";

import type { CatalogOptions } from "@/lib/api";

export const emptyOptions: CatalogOptions = {
  authors: [],
  publishers: [],
  owners: [],
  locations: [],
};

/** Names already in the catalog, for autocompletion and filters. */
export const CatalogOptionsContext = createContext<CatalogOptions>(emptyOptions);

export function useCatalogOptions() {
  return useContext(CatalogOptionsContext);
}
