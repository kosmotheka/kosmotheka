import { useTranslation } from "react-i18next";
import { Check } from "lucide-react";

import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { cn } from "@/lib/utils";

/** Ticks which libraries go to another device. Nothing is sent unless ticked. */
export function LibraryChecklist({
  selected,
  onChange,
  disabled = false,
}: {
  selected: string[];
  onChange: (selected: string[]) => void;
  disabled?: boolean;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const toggle = (id: string) =>
    onChange(selected.includes(id) ? selected.filter((s) => s !== id) : [...selected, id]);

  return (
    <div className="-mx-2 flex flex-col">
      {libraries.all.map((library) => {
        const on = selected.includes(library.id);
        return (
          <button
            key={library.id}
            type="button"
            role="checkbox"
            aria-checked={on}
            disabled={disabled}
            className="flex min-h-14 items-center gap-4 rounded-xl px-2 text-left hover:bg-muted disabled:opacity-60"
            onClick={() => toggle(library.id)}
          >
            <span
              className={cn(
                "flex size-5 shrink-0 items-center justify-center rounded-md border-[1.5px]",
                on ? "border-primary bg-primary text-primary-foreground" : "border-muted-foreground",
              )}
            >
              {on && <Check className="size-3.5" strokeWidth={3} />}
            </span>
            <span className="flex min-w-0 flex-col gap-0.5">
              <span className="truncate text-[15px] font-medium">{nameOf(library)}</span>
              <span className="text-[13px] text-muted-foreground">
                {t("nav.works", { count: library.works })}
              </span>
            </span>
          </button>
        );
      })}
    </div>
  );
}

/** "Will send: A, B. Nothing else leaves this device." */
export function SendSummary({ selected }: { selected: string[] }) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const names = libraries.all
    .filter((library) => selected.includes(library.id))
    .map((library) => nameOf(library));
  return (
    <p className="text-[13px] text-pretty text-muted-foreground">
      {names.length > 0
        ? t("sharing.willSend", { names: names.join(", ") })
        : t("sharing.sendNothing")}
    </p>
  );
}
