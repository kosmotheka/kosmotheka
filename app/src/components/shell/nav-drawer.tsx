import { useTranslation } from "react-i18next";
import { LibraryBig, Plus } from "lucide-react";

import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { MenuList } from "@/components/shell/menu-list";
import { appNav, libraryNav } from "@/components/shell/nav-items";
import { type useSyncStatus, SyncStatus } from "@/components/shell/sync-status";
import { Sheet, SheetContent, SheetDescription, SheetTitle } from "@/components/ui/sheet";
import { navigate, closeOverlay, openOverlay } from "@/lib/navigation";
import { cn } from "@/lib/utils";

/** Mobile side menu: the user's libraries, what belongs to the current one, and at the bottom what is general. */
export function NavDrawer({
  open,
  count,
  sync,
}: {
  open: boolean;
  count: number;
  sync: ReturnType<typeof useSyncStatus>;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const choose = async (id: string) => {
    if (id !== libraries.active?.id) await libraries.switchTo(id);
    navigate("catalog");
  };
  return (
    <Sheet open={open} onOpenChange={(next) => !next && closeOverlay()}>
      <SheetContent
        side="left"
        showCloseButton={false}
        className="w-[316px] gap-0 rounded-r-[20px] bg-sidebar px-3 pt-[max(2.75rem,env(safe-area-inset-top))] pb-[max(1.25rem,env(safe-area-inset-bottom))] text-sidebar-foreground"
      >
        <SheetTitle className="px-4 pt-2 pb-4 text-xl font-semibold tracking-tight">
          {t("app.title")}
        </SheetTitle>
        <SheetDescription className="sr-only">{t("nav.menu")}</SheetDescription>
        <div className="px-4 py-1.5 text-xs font-medium text-muted-foreground">
          {t("nav.libraries")}
        </div>
        {libraries.all.map((item) => (
          <button
            key={item.id}
            type="button"
            aria-current={item.active || undefined}
            className={cn(
              "flex h-12 w-full items-center gap-3 rounded-full px-4 text-left text-sm",
              item.active ? "bg-sidebar-accent font-semibold" : "hover:bg-sidebar-accent/60",
            )}
            onClick={() => void choose(item.id)}
          >
            <LibraryBig className="size-5 shrink-0" />
            <span className="flex-1 truncate">{nameOf(item)}</span>
            <span className="text-xs font-normal text-muted-foreground tabular-nums">
              {item.active ? count : item.works}
            </span>
          </button>
        ))}
        <button
          type="button"
          className="flex h-12 w-full items-center gap-3 rounded-full px-4 text-left text-sm hover:bg-sidebar-accent/60"
          onClick={() => openOverlay({ name: "newLibrary" })}
        >
          <Plus className="size-5 shrink-0" />
          {t("nav.newLibrary")}
        </button>
        <div className="mx-4 my-2 h-px bg-border" />
        {/* As in the desktop sidebar: what belongs to this library under its name, what is general at the bottom. */}
        <div className="flex flex-1 flex-col overflow-y-auto">
          <div className="truncate px-4 py-1.5 text-xs font-medium text-muted-foreground">
            {libraries.active ? nameOf(libraries.active) : null}
          </div>
          <MenuList items={libraryNav} size="large" />
          <span className="min-h-4 flex-1" />
          <div className="mx-4 mb-2 h-px shrink-0 bg-border" />
          <MenuList items={appNav} size="large" />
        </div>
        <SyncStatus status={sync} className="px-4 pt-3" />
      </SheetContent>
    </Sheet>
  );
}
