import { useTranslation } from "react-i18next";

import { go, mainNav } from "@/components/shell/nav-items";
import { type Section } from "@/lib/navigation";
import { cn } from "@/lib/utils";

/**
 * Mobile bottom bar with three destinations. "Do I have it?" sits in the middle
 * as the raised main action: it opens the camera flow, not a section.
 */
export function BottomNav({ section }: { section: Section }) {
  const { t } = useTranslation();
  const [catalog, check, missing] = mainNav;

  const tab = (item: typeof catalog) => {
    const active = item.section === section;
    const Icon = item.icon;
    return (
      <button
        type="button"
        aria-current={active ? "page" : undefined}
        className="flex flex-col items-center gap-1 pt-3"
        onClick={() => go(item)}
      >
        <span
          className={cn(
            "flex h-8 w-16 items-center justify-center rounded-full",
            active ? "bg-secondary text-foreground" : "text-muted-foreground",
          )}
        >
          <Icon className="size-[22px]" strokeWidth={active ? 2.2 : 2} />
        </span>
        <span
          className={cn(
            "text-xs",
            active ? "font-semibold text-foreground" : "font-medium text-muted-foreground",
          )}
        >
          {t(item.label)}
        </span>
      </button>
    );
  };

  const CheckIcon = check.icon;
  return (
    <nav
      aria-label={t("nav.menu")}
      className="shrink-0 border-t bg-background pb-[max(1.125rem,env(safe-area-inset-bottom))]"
    >
      <div className="grid h-[76px] grid-cols-3 items-start">
        {tab(catalog)}
        <button
          type="button"
          className="-mt-[18px] flex flex-col items-center gap-1"
          onClick={() => go(check)}
        >
          <span className="flex size-[60px] items-center justify-center rounded-full bg-primary text-primary-foreground shadow-[0_0_0_5px_var(--background),0_6px_16px_-4px_oklch(0_0_0/30%)]">
            <CheckIcon className="size-[26px]" />
          </span>
          <span className="text-xs font-semibold">{t(check.label)}</span>
        </button>
        {tab(missing)}
      </div>
    </nav>
  );
}
