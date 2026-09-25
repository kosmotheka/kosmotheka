import { useTranslation } from "react-i18next";
import { LibraryBig } from "lucide-react";

import { MenuList } from "@/components/shell/menu-list";
import { appNav, libraryNav } from "@/components/shell/nav-items";
import { type useSyncStatus, SyncStatus } from "@/components/shell/sync-status";
import { Sheet, SheetContent, SheetDescription, SheetTitle } from "@/components/ui/sheet";
import { closeOverlay } from "@/lib/navigation";

/** Mobile side menu: the user's libraries, then what belongs to the current one. */
export function NavDrawer({
  open,
  library,
  count,
  sync,
}: {
  open: boolean;
  library: string;
  count: number;
  sync: ReturnType<typeof useSyncStatus>;
}) {
  const { t } = useTranslation();
  return (
    <Sheet open={open} onOpenChange={(next) => !next && closeOverlay()}>
      <SheetContent
        side="left"
        showCloseButton={false}
        className="w-[316px] gap-0 rounded-r-[20px] bg-sidebar px-3 pt-[max(2.75rem,env(safe-area-inset-top))] pb-5 text-sidebar-foreground"
      >
        <SheetTitle className="px-4 pt-2 pb-4 text-xl font-semibold tracking-tight">
          {t("app.title")}
        </SheetTitle>
        <SheetDescription className="sr-only">{t("nav.menu")}</SheetDescription>
        <div className="px-4 py-1.5 text-xs font-medium text-muted-foreground">
          {t("nav.libraries")}
        </div>
        <div className="flex h-12 items-center gap-3 rounded-full bg-sidebar-accent px-4 text-sm font-semibold">
          <LibraryBig className="size-5" />
          <span className="flex-1">{library}</span>
          <span className="text-xs font-normal text-muted-foreground tabular-nums">{count}</span>
        </div>
        <div className="mx-4 my-2 h-px bg-border" />
        <div className="flex-1 overflow-y-auto">
          <MenuList items={[...libraryNav, ...appNav]} size="large" />
        </div>
        <SyncStatus status={sync} className="px-4 pt-3" />
      </SheetContent>
    </Sheet>
  );
}
