import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";

/** "In this library / In all of them", for searches. */
export function SearchScope({
  library,
  everywhere,
  onChange,
  compact = false,
}: {
  library: string;
  everywhere: boolean;
  onChange: (everywhere: boolean) => void;
  /** Inside the desktop search box rather than below it. */
  compact?: boolean;
}) {
  const { t } = useTranslation();
  const option = (value: boolean, label: string) => (
    <button
      type="button"
      role="radio"
      aria-checked={everywhere === value}
      className={cn(
        "flex items-center justify-center truncate px-2.5",
        compact ? "h-[26px] rounded-[5px] text-xs" : "h-10 rounded-[10px] text-sm",
        everywhere === value ? "bg-background font-semibold shadow-sm" : "text-muted-foreground",
      )}
      onClick={() => onChange(value)}
    >
      {label}
    </button>
  );
  return (
    <div
      role="radiogroup"
      aria-label={t("libraries.searchWhere")}
      className={cn(
        "grid shrink-0 grid-cols-2 bg-muted",
        compact ? "rounded-md p-0.5" : "rounded-[14px] p-1",
      )}
    >
      {option(false, t("libraries.inThis", { name: library }))}
      {option(true, t("libraries.inAll"))}
    </div>
  );
}
