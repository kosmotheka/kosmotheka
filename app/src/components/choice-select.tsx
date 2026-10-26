import { useDesktop } from "@/components/shell/use-desktop";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";

export interface Choice {
  value: string;
  label: string;
}

/**
 * A list to pick one from: the system's own picker on a phone, where it opens reliably and a
 * long list scrolls (the menus tended to close by themselves the first time on Android), and
 * the app's menu on the desktop.
 */
export function ChoiceSelect({
  id,
  value,
  onChange,
  choices,
  placeholder,
  className,
}: {
  id?: string;
  /** Empty while nothing is chosen yet, which shows `placeholder`. */
  value: string;
  onChange: (value: string) => void;
  choices: Choice[];
  placeholder?: string;
  className?: string;
}) {
  const desktop = useDesktop();
  if (!desktop) {
    return (
      <select
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={cn(
          "h-9 w-full min-w-0 rounded-lg border border-input bg-transparent px-2 text-base outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 dark:bg-input/30",
          className,
        )}
      >
        {value === "" && (
          <option value="" disabled>
            {placeholder}
          </option>
        )}
        {choices.map((choice) => (
          <option key={choice.value} value={choice.value}>
            {choice.label}
          </option>
        ))}
      </select>
    );
  }
  return (
    <Select value={value || undefined} onValueChange={onChange}>
      <SelectTrigger id={id} className={cn("w-full", className)}>
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>
      <SelectContent position="popper" className="max-h-72">
        {choices.map((choice) => (
          <SelectItem key={choice.value} value={choice.value}>
            {choice.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
