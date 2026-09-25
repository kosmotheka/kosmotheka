import { useTranslation } from "react-i18next";

import { type NavItem, go } from "@/components/shell/nav-items";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

/** A column of menu entries; the active one is highlighted, planned ones disabled. */
export function MenuList({
  items,
  activeId,
  counts = {},
  size = "default",
}: {
  items: NavItem[];
  activeId?: string;
  counts?: Record<string, number>;
  /** "large" for the mobile drawer (48 px rows), "default" for the desktop sidebar. */
  size?: "default" | "large";
}) {
  const { t } = useTranslation();
  return (
    <nav className="flex flex-col gap-0.5">
      {items.map((item) => {
        const Icon = item.icon;
        const active = item.id === activeId;
        return (
          <button
            key={item.id}
            type="button"
            disabled={item.soon}
            aria-current={active ? "page" : undefined}
            onClick={() => go(item)}
            className={cn(
              "flex items-center gap-3 text-left text-sm disabled:cursor-default disabled:text-muted-foreground",
              size === "large" ? "h-12 rounded-full px-4" : "h-9 rounded-lg px-2.5",
              active ? "bg-sidebar-accent font-semibold" : "hover:bg-sidebar-accent/60 disabled:hover:bg-transparent",
            )}
          >
            <Icon className={size === "large" ? "size-5" : "size-[17px]"} />
            <span className="flex-1">{t(item.label)}</span>
            {item.soon && (
              <Badge variant="outline" className="font-normal">
                {t("nav.soon")}
              </Badge>
            )}
            {counts[item.id] !== undefined && (
              <span className="text-xs font-normal text-muted-foreground tabular-nums">
                {counts[item.id]}
              </span>
            )}
          </button>
        );
      })}
    </nav>
  );
}
