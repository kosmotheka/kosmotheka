import { type ComponentProps, type ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, ChevronDown, Plus, Search } from "lucide-react";

import { useDesktop } from "@/components/shell/use-desktop";
import { Input } from "@/components/ui/input";
import { Sheet, SheetContent, SheetDescription, SheetTitle } from "@/components/ui/sheet";
import { useBackGuard } from "@/lib/navigation";
import { cn } from "@/lib/utils";

/** Lower case without accents: «mino» finds «Minotauro», «janes» finds «Plaza & Janés». */
const plain = (text: string) =>
  text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();

/** The known values that hold what was typed, those that start with it first; all of them before typing. */
function matching(values: string[], typed: string): string[] {
  const query = plain(typed.trim());
  if (!query) return values;
  const found = values.filter((value) => plain(value).includes(query));
  const starts = found.filter((value) => plain(value).startsWith(query));
  const inside = found.filter((value) => !plain(value).startsWith(query));
  return [...starts, ...inside];
}

const isKnown = (values: string[], typed: string) =>
  values.some((value) => plain(value) === plain(typed.trim()));

/**
 * The phone's way to fill a field from the catalog's known values (authors, publishers, owners, places, tags):
 * a sheet from below, like the genres', with every known value, a search box over them and a row to use what
 * was typed as a new one. Unlike suggestions that wait for typing, it shows at once whether there are any.
 */
export function PickSheet({
  open,
  onOpenChange,
  title,
  values,
  current,
  onPick,
  newLabel,
  onClear,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  values: string[];
  /** The value the field has now, marked in the list. */
  current?: string;
  onPick: (value: string) => void;
  /** What the row for a new value says; the tags' says it comes back from the trash. */
  newLabel?: (typed: string) => string;
  /** Leaves the field empty; only for a field that has a value. */
  onClear?: () => void;
}) {
  const { t } = useTranslation();
  const [typed, setTyped] = useState("");
  const close = () => {
    setTyped("");
    onOpenChange(false);
  };
  // Back closes the sheet, not the form under it.
  useBackGuard(open, () => {
    close();
    return false;
  });
  const pick = (value: string) => {
    const trimmed = value.trim();
    if (trimmed) onPick(trimmed);
    close();
  };
  const shown = matching(values, typed);
  const offerNew = typed.trim() !== "" && !isKnown(values, typed);

  return (
    <Sheet open={open} onOpenChange={(next) => (next ? onOpenChange(true) : close())}>
      <SheetContent
        side="bottom"
        showCloseButton={false}
        className="max-h-[85dvh] gap-3 rounded-t-[28px] bg-popover px-5 pt-2.5 pb-[max(1.25rem,env(safe-area-inset-bottom))]"
      >
        <div className="mx-auto h-1 w-8 rounded-full bg-muted-foreground/40" aria-hidden />
        <div className="flex items-baseline justify-between gap-3">
          <SheetTitle className="text-lg font-semibold">{title}</SheetTitle>
          <SheetDescription className="sr-only">{t("pick.description")}</SheetDescription>
          {onClear && (
            <button
              type="button"
              className="text-sm font-medium text-primary"
              onClick={() => {
                onClear();
                close();
              }}
            >
              {t("pick.clear")}
            </button>
          )}
        </div>
        <div className="relative">
          <Search className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            className="h-11 pl-9"
            placeholder={t("pick.search")}
            aria-label={t("pick.search")}
            autoComplete="off"
            value={typed}
            onChange={(event) => setTyped(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                pick(shown[0] ?? typed);
              }
            }}
          />
        </div>
        <div role="listbox" aria-label={title} className="-mx-2 flex flex-col overflow-y-auto">
          {shown.map((value) => (
            <button
              key={value}
              type="button"
              role="option"
              aria-selected={value === current}
              className="flex min-h-12 items-center gap-3.5 rounded-lg px-2 text-left text-[15px] active:bg-muted"
              onClick={() => pick(value)}
            >
              <Check className={cn("size-5 shrink-0", value !== current && "invisible")} />
              {value}
            </button>
          ))}
          {offerNew && (
            <button
              type="button"
              role="option"
              aria-selected={false}
              className="flex min-h-12 items-center gap-3.5 rounded-lg px-2 text-left text-[15px] font-medium text-primary active:bg-muted"
              onClick={() => pick(typed)}
            >
              <Plus className="size-5 shrink-0" />
              {newLabel ? newLabel(typed.trim()) : t("pick.add", { value: typed.trim() })}
            </button>
          )}
          {values.length === 0 && !offerNew && (
            <p className="px-2 py-3 text-sm text-muted-foreground">{t("pick.none")}</p>
          )}
          {values.length > 0 && shown.length === 0 && !offerNew && (
            <p className="px-2 py-3 text-sm text-muted-foreground">{t("pick.noMatch")}</p>
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}

/**
 * On the computer, the catalog's known values under a field, from the moment it gets the focus, filtered as it
 * is typed in; drawn by the app, not a `<datalist>`. The field goes inside an element with `relative`, and
 * spreads `inputProps`.
 */
export function useSuggestions(values: string[], typed: string, pick: (value: string) => void) {
  const [open, setOpen] = useState(false);
  // On entering, every value, even with one already in the field; typing filters them.
  const [filtering, setFiltering] = useState(false);
  const shown = open
    ? matching(values, filtering ? typed : "").filter((value) => plain(value) !== plain(typed))
    : [];
  const inputProps = {
    autoComplete: "off",
    onFocus: () => {
      setOpen(true);
      setFiltering(false);
    },
    // After a click on a suggestion, which keeps the focus anyway (`onMouseDown` below).
    onBlur: () => setTimeout(() => setOpen(false), 100),
    onInput: () => {
      setOpen(true);
      setFiltering(true);
    },
    onKeyDown: (event: { key: string }) => {
      if (event.key === "Escape") setOpen(false);
    },
  };
  const list: ReactNode =
    shown.length > 0 ? (
      <div
        role="listbox"
        className="absolute top-full left-0 z-10 mt-1 max-h-64 w-full max-w-sm overflow-y-auto rounded-lg border bg-popover p-1 text-popover-foreground shadow-md"
      >
        {shown.map((value) => (
          <button
            key={value}
            type="button"
            role="option"
            aria-selected={false}
            className="flex min-h-9 w-full items-center rounded-md px-2 text-left text-sm hover:bg-accent"
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => {
              pick(value);
              setOpen(false);
            }}
          >
            {value}
          </button>
        ))}
      </div>
    ) : null;
  return { inputProps, list };
}

/**
 * A field filled from the catalog's known values: the list under it on the computer, a sheet with a search box
 * on the phone. `label` titles the sheet.
 */
export function SuggestInput({
  suggestions,
  value,
  onValueChange,
  label,
  className,
  placeholder,
  id,
  ...props
}: Omit<ComponentProps<typeof Input>, "value" | "onChange" | "list"> & {
  suggestions: string[];
  value: string;
  onValueChange: (value: string) => void;
  label: string;
}) {
  const { t } = useTranslation();
  const desktop = useDesktop();
  const [picking, setPicking] = useState(false);
  const { inputProps, list } = useSuggestions(suggestions, value, onValueChange);

  if (desktop) {
    return (
      <div className={cn("relative", className)}>
        <Input
          {...props}
          {...inputProps}
          id={id}
          placeholder={placeholder}
          value={value}
          onChange={(event) => onValueChange(event.target.value)}
        />
        {list}
      </div>
    );
  }
  return (
    <div className={className}>
      <button
        id={id}
        type="button"
        // As tall as the fields and the language picker beside it.
        className="flex h-8 w-full min-w-0 items-center gap-2 rounded-lg border border-input bg-transparent px-2 text-left text-base outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 dark:bg-input/30"
        onClick={() => setPicking(true)}
      >
        <span className={cn("min-w-0 flex-1 truncate", !value && "text-muted-foreground")}>
          {value || placeholder || t("pick.choose")}
        </span>
        <ChevronDown className="size-4 shrink-0 text-muted-foreground" />
      </button>
      <PickSheet
        open={picking}
        onOpenChange={setPicking}
        title={label}
        values={suggestions}
        current={value}
        onPick={onValueChange}
        onClear={value ? () => onValueChange("") : undefined}
      />
    </div>
  );
}
