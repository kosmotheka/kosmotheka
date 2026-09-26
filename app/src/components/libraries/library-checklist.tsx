import { Trans, useTranslation } from "react-i18next";
import { Check, ShieldCheck } from "lucide-react";

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
    <div className="flex flex-col divide-y overflow-hidden rounded-xl border">
      {libraries.all.map((library) => {
        const on = selected.includes(library.id);
        return (
          <button
            key={library.id}
            type="button"
            role="checkbox"
            aria-checked={on}
            disabled={disabled}
            className="flex min-h-14 items-center gap-3.5 px-3.5 py-2 text-left hover:bg-muted disabled:opacity-60"
            onClick={() => toggle(library.id)}
          >
            <Tick on={on} />
            <span className="flex min-w-0 flex-col gap-0.5">
              <span className="truncate text-[15px] font-medium">{nameOf(library)}</span>
              <span className="text-[13px] text-muted-foreground">
                {library.active
                  ? t("sharing.activeHint", { works: t("nav.works", { count: library.works }) })
                  : t("nav.works", { count: library.works })}
              </span>
            </span>
          </button>
        );
      })}
    </div>
  );
}

/** A square tick box, drawn to match Design's. */
export function Tick({ on }: { on: boolean }) {
  return (
    <span
      className={cn(
        "flex size-5 shrink-0 items-center justify-center rounded-md border-[1.5px]",
        on ? "border-primary bg-primary text-primary-foreground" : "border-muted-foreground",
      )}
    >
      {on && <Check className="size-3.5" strokeWidth={3} />}
    </span>
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
    <p className="flex items-start gap-2 text-[13px] text-pretty text-muted-foreground">
      <ShieldCheck className="mt-0.5 size-4 shrink-0" />
      <span>
        {names.length > 0 ? (
          <Trans
            i18nKey="sharing.willSend"
            values={{ names: names.join(", ") }}
            components={{ strong: <strong className="font-medium text-foreground" /> }}
          />
        ) : (
          t("sharing.sendNothing")
        )}
      </span>
    </p>
  );
}
