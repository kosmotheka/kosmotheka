import { useTranslation } from "react-i18next";
import { ChevronDown, Menu, Search } from "lucide-react";

import { type useSyncStatus, SyncStatus } from "@/components/shell/sync-status";
import { Button } from "@/components/ui/button";
import { openOverlay } from "@/lib/navigation";

/** Mobile top bar: menu, library selector with the sync state, and search. */
export function AppBar({
  library,
  sync,
  onSearch,
}: {
  library: string;
  sync: ReturnType<typeof useSyncStatus>;
  onSearch: () => void;
}) {
  const { t } = useTranslation();
  return (
    <header className="grid h-16 shrink-0 grid-cols-[48px_1fr_48px] items-center px-1">
      <Button
        variant="ghost"
        size="icon-lg"
        className="size-12 rounded-full"
        aria-label={t("nav.openMenu")}
        onClick={() => openOverlay({ name: "drawer" })}
      >
        <Menu className="size-[22px]" />
      </Button>
      <button
        type="button"
        aria-label={t("nav.changeLibrary")}
        className="flex flex-col items-center gap-px justify-self-center rounded-xl px-3 py-1 hover:bg-muted"
        onClick={() => openOverlay({ name: "libraries" })}
      >
        <span className="flex items-center gap-1 text-[17px] font-semibold tracking-tight">
          {library}
          <ChevronDown className="size-4" />
        </span>
        <SyncStatus status={sync} className="text-[11px]" />
      </button>
      <Button
        variant="ghost"
        size="icon-lg"
        className="size-12 rounded-full"
        aria-label={t("nav.search")}
        onClick={onSearch}
      >
        <Search className="size-[21px]" />
      </Button>
    </header>
  );
}
