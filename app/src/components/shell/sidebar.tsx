import { useTranslation } from "react-i18next";
import { ChevronsUpDown, LibraryBig } from "lucide-react";

import { MenuList } from "@/components/shell/menu-list";
import { appNav, libraryNav, mainNav } from "@/components/shell/nav-items";
import { type Section, openOverlay } from "@/lib/navigation";

/** Desktop sidebar: library selector, the three destinations, then the menu. */
export function Sidebar({
  section,
  library,
  counts,
}: {
  section: Section;
  library: string;
  counts: { catalog: number; missing: number };
}) {
  const { t } = useTranslation();
  return (
    <aside
      aria-label={t("nav.menu")}
      className="flex h-full w-64 shrink-0 flex-col border-r border-sidebar-border bg-sidebar p-3 text-sidebar-foreground"
    >
      <button
        type="button"
        aria-label={t("nav.changeLibrary")}
        className="flex items-center gap-2.5 rounded-lg p-2 text-left hover:bg-sidebar-accent"
        onClick={() => openOverlay({ name: "libraries" })}
      >
        <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-primary text-primary-foreground">
          <LibraryBig className="size-[17px]" />
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="text-sm font-semibold">{library}</span>
          <span className="text-xs text-muted-foreground">
            {t("nav.works", { count: counts.catalog })}
          </span>
        </span>
        <ChevronsUpDown className="size-4" />
      </button>
      <div className="mt-4">
        <MenuList items={mainNav} activeId={section} counts={counts} />
      </div>
      <div className="px-2.5 pt-4.5 pb-1.5 text-xs font-medium text-muted-foreground">
        {t("nav.library")}
      </div>
      <MenuList items={libraryNav} />
      <span className="flex-1" />
      <MenuList items={appNav} activeId={section} />
    </aside>
  );
}
