import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ChevronRight, LibraryBig, Plus } from "lucide-react";

import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { MenuList } from "@/components/shell/menu-list";
import { appNav, libraryNav } from "@/components/shell/nav-items";
import { type useSyncStatus, SyncStatus } from "@/components/shell/sync-status";
import { Sheet, SheetContent, SheetDescription, SheetTitle } from "@/components/ui/sheet";
import { navigate, closeOverlay, openOverlay } from "@/lib/navigation";
import { cn } from "@/lib/utils";

// The libraries this device opened last, newest first: a per-device convenience, never synced.
const RECENT_KEY = "kosmotheka.recentLibraries";
// The menu shows the open library and the last ones opened; the rest are one tap away.
const SHOWN = 3;

function recentIds(): string[] {
  try {
    const stored: unknown = JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
    return Array.isArray(stored) ? stored.filter((id): id is string => typeof id === "string") : [];
  } catch {
    return [];
  }
}

function rememberOpened(id: string) {
  try {
    const ids = [id, ...recentIds().filter((other) => other !== id)].slice(0, 20);
    localStorage.setItem(RECENT_KEY, JSON.stringify(ids));
  } catch {
    // Without storage the menu just shows the first ones.
  }
}

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
  const activeId = libraries.active?.id;
  useEffect(() => {
    if (activeId) rememberOpened(activeId);
  }, [activeId]);
  // The open one first, then the last opened, then the rest in their usual order.
  const recent = recentIds();
  const rank = (id: string) =>
    id === activeId ? -1 : recent.indexOf(id) === -1 ? 999 : recent.indexOf(id);
  const shown = [...libraries.all].sort((a, b) => rank(a.id) - rank(b.id)).slice(0, SHOWN);
  const hidden = libraries.all.length - shown.length;
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
        {/* One scroll for the whole menu, libraries included. */}
        <div className="-mx-3 flex min-h-0 flex-1 flex-col overflow-y-auto px-3">
          <div className="px-4 py-1.5 text-xs font-medium text-muted-foreground">
            {t("nav.libraries")}
          </div>
          {shown.map((item) => (
            <button
              key={item.id}
              type="button"
              aria-current={item.active || undefined}
              className={cn(
                "flex min-h-12 w-full shrink-0 items-center gap-3 rounded-full px-4 py-2 text-left text-sm",
                item.active ? "bg-sidebar-accent font-semibold" : "hover:bg-sidebar-accent/60",
              )}
              onClick={() => void choose(item.id)}
            >
              <LibraryBig className="size-5 shrink-0" />
              <span className="line-clamp-2 flex-1 break-words">{nameOf(item)}</span>
              <span className="text-xs font-normal text-muted-foreground tabular-nums">
                {item.active ? count : item.works}
              </span>
            </button>
          ))}
          {hidden > 0 && (
            <button
              type="button"
              className="flex h-12 w-full shrink-0 items-center gap-3 rounded-full px-4 text-left text-sm hover:bg-sidebar-accent/60"
              onClick={() => openOverlay({ name: "libraries" })}
            >
              <span className="size-5 shrink-0" aria-hidden />
              <span className="flex-1">
                {t("nav.allLibraries", { count: libraries.all.length })}
              </span>
              <ChevronRight className="size-4 text-muted-foreground" />
            </button>
          )}
          <button
            type="button"
            className="flex h-12 w-full shrink-0 items-center gap-3 rounded-full px-4 text-left text-sm hover:bg-sidebar-accent/60"
            onClick={() => openOverlay({ name: "newLibrary" })}
          >
            <Plus className="size-5 shrink-0" />
            {t("nav.newLibrary")}
          </button>
          <div className="mx-4 my-2 h-px shrink-0 bg-border" />
          {/* As in the desktop sidebar: what belongs to this library under its name, what is general at the bottom. */}
          <div className="flex flex-1 flex-col">
            <div className="px-4 py-1.5 text-xs font-medium break-words text-muted-foreground">
              {libraries.active ? nameOf(libraries.active) : null}
            </div>
            <MenuList items={libraryNav} size="large" />
            <span className="min-h-4 flex-1" />
            <div className="mx-4 mb-2 h-px shrink-0 bg-border" />
            <MenuList items={appNav} size="large" />
          </div>
        </div>
        <SyncStatus status={sync} className="px-4 pt-3" />
      </SheetContent>
    </Sheet>
  );
}
