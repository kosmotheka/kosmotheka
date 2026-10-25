import { useTranslation } from "react-i18next";
import { BookCheck, Plus, Search, Send, Zap } from "lucide-react";

import { useLibraries } from "@/components/libraries-context";
import { useRequests } from "@/components/requests-context";
import { SearchScope } from "@/components/search-scope";
import { RequestsBell } from "@/components/shell/requests-bell";
import { type useSyncStatus, SyncStatus } from "@/components/shell/sync-status";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { navigate, openOverlay } from "@/lib/navigation";

/** Desktop top bar: search, sync state and the main actions. */
export function TopBar({
  library,
  query,
  onQueryChange,
  everywhere,
  onEverywhereChange,
  sync,
  missing = false,
}: {
  library: string;
  query: string;
  onQueryChange: (query: string) => void;
  everywhere: boolean;
  onEverywhereChange: (everywhere: boolean) => void;
  sync: ReturnType<typeof useSyncStatus>;
  /** In the «Me falta» view: «Añadir» catalogs without a copy (D61). */
  missing?: boolean;
}) {
  const libraries = useLibraries();
  // Readers ask for books instead of adding them (D73).
  const reader = useRequests().myRole === "reader";
  const { t } = useTranslation();
  return (
    <header className="flex h-14 shrink-0 items-center gap-2 border-b bg-background pr-4 pl-5">
      <div role="search" className="flex max-w-[520px] min-w-0 flex-1 items-center gap-2">
        <div className="relative max-w-[360px] min-w-0 flex-1">
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
        {query.trim() && libraries.all.length > 1 && (
          <SearchScope
            compact
            library={library}
            everywhere={everywhere}
            onChange={onEverywhereChange}
          />
        )}
      </div>
      <span className="flex-1" />
      <SyncStatus status={sync} asButton compact />
      <RequestsBell />
      <span className="mx-1 h-5 w-px bg-border" aria-hidden />
      <Button
        variant="outline"
        size="lg"
        aria-label={t("nav.check")}
        title={t("nav.check")}
        onClick={() => navigate("check")}
      >
        <BookCheck /> <span className="hidden xl:inline">{t("nav.check")}</span>
      </Button>
      {reader ? (
        <Button
          size="lg"
          onClick={() => openOverlay(missing ? { name: "add", missing: true } : { name: "add" })}
        >
          <Send /> {t("ask.add")}
        </Button>
      ) : (
        <>
          {!missing && (
            <Button
              variant="outline"
              size="lg"
              aria-label={t("nav.quickAdd")}
              title={t("nav.quickAdd")}
              onClick={() => openOverlay({ name: "burst" })}
            >
              <Zap /> <span className="hidden xl:inline">{t("nav.quickAdd")}</span>
            </Button>
          )}
          <Button
            size="lg"
            onClick={() => openOverlay(missing ? { name: "add", missing: true } : { name: "add" })}
          >
            <Plus /> {t("nav.add")}
          </Button>
        </>
      )}
    </header>
  );
}
