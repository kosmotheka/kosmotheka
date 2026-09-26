import { useTranslation } from "react-i18next";
import { Plus, ScanBarcode, Search, Zap } from "lucide-react";

import { type useSyncStatus, SyncStatus } from "@/components/shell/sync-status";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { navigate, openOverlay } from "@/lib/navigation";

/** Desktop top bar: search, sync state and the main actions. */
export function TopBar({
  library,
  query,
  onQueryChange,
  sync,
}: {
  library: string;
  query: string;
  onQueryChange: (query: string) => void;
  sync: ReturnType<typeof useSyncStatus>;
}) {
  const { t } = useTranslation();
  return (
    <header className="flex h-14 shrink-0 items-center gap-2 border-b bg-background pr-4 pl-5">
      <div role="search" className="relative w-[360px]">
        <Search className="absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          id="catalog-search"
          type="search"
          aria-label={t("filters.search")}
          placeholder={t("nav.searchIn", { library })}
          className="h-9 pl-8"
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
        />
      </div>
      <span className="flex-1" />
      <SyncStatus status={sync} asButton />
      <span className="mx-1 h-5 w-px bg-border" aria-hidden />
      <Button variant="outline" size="lg" onClick={() => navigate("check")}>
        <ScanBarcode /> {t("nav.scan")}
      </Button>
      <Button variant="outline" size="lg" onClick={() => openOverlay({ name: "burst" })}>
        <Zap /> {t("nav.quickAdd")}
      </Button>
      <Button size="lg" onClick={() => openOverlay({ name: "add" })}>
        <Plus /> {t("nav.add")}
      </Button>
    </header>
  );
}
