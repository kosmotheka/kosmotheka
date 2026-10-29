import { type ComponentProps, type ReactNode, useState } from "react";

import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

/** How many suggestions show at once: the rest are found by typing more. */
const SHOWN = 8;

/** Lower case without accents: «mino» finds «Minotauro», «janes» finds «Plaza & Janés». */
const plain = (text: string) =>
  text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();

/** The known values that hold what was typed, those that start with it first; nothing until something is typed. */
function matching(values: string[], typed: string): string[] {
  const query = plain(typed.trim());
  if (!query) return [];
  const found = values.filter((value) => {
    const known = plain(value);
    return known !== query && known.includes(query);
  });
  const starts = found.filter((value) => plain(value).startsWith(query));
  const inside = found.filter((value) => !plain(value).startsWith(query));
  return [...starts, ...inside].slice(0, SHOWN);
}

/**
 * The catalog's known values under a field as it is typed in (authors, publishers, owners, places), drawn by
 * the app: Android's WebView draws a `<datalist>` out of place and half transparent. The field goes inside an
 * element with `relative`, and spreads `inputProps`.
 */
export function useSuggestions(values: string[], typed: string, pick: (value: string) => void) {
  const [open, setOpen] = useState(false);
  const shown = open ? matching(values, typed) : [];
  const inputProps = {
    autoComplete: "off",
    onFocus: () => setOpen(true),
    // After a tap on a suggestion, which keeps the focus anyway (`onMouseDown` below).
    onBlur: () => setTimeout(() => setOpen(false), 100),
    onInput: () => setOpen(true),
    onKeyDown: (event: { key: string }) => {
      if (event.key === "Escape") setOpen(false);
    },
  };
  const list: ReactNode =
    shown.length > 0 ? (
      <div
        role="listbox"
        className="absolute top-full left-0 z-10 mt-1 w-full max-w-sm rounded-lg border bg-popover p-1 text-popover-foreground shadow-md"
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

/** A text field with the catalog's known values under it as it is typed in. */
export function SuggestInput({
  suggestions,
  value,
  onValueChange,
  className,
  ...props
}: Omit<ComponentProps<typeof Input>, "value" | "onChange" | "list"> & {
  suggestions: string[];
  value: string;
  onValueChange: (value: string) => void;
}) {
  const { inputProps, list } = useSuggestions(suggestions, value, onValueChange);
  return (
    <div className={cn("relative", className)}>
      <Input
        {...props}
        {...inputProps}
        value={value}
        onChange={(event) => onValueChange(event.target.value)}
      />
      {list}
    </div>
  );
}
