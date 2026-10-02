import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";

/**
 * The 6-digit code as six boxes (Auth.dc.html): one real input overlaid on top,
 * so it stays a single accessible text field for autofill and screen readers.
 */
export function CodeInput({
  value,
  onChange,
  invalid,
  disabled,
}: {
  value: string;
  onChange: (value: string) => void;
  invalid?: boolean;
  disabled?: boolean;
}) {
  const { t } = useTranslation();
  const digits = Array.from({ length: 6 }, (_, i) => value[i] ?? "");

  return (
    <div className="relative flex gap-2">
      <input
        aria-label={t("account.auth.code")}
        inputMode="numeric"
        autoComplete="one-time-code"
        maxLength={6}
        disabled={disabled}
        value={value}
        onChange={(e) => onChange(e.target.value.replace(/\D/g, "").slice(0, 6))}
        className="absolute inset-0 h-14 w-full cursor-text opacity-0"
      />
      {digits.map((digit, i) => (
        <span
          key={i}
          aria-hidden="true"
          className={cn(
            "flex h-14 flex-1 items-center justify-center rounded-xl border font-mono text-[22px] font-semibold",
            invalid ? "border-destructive border-[1.5px]" : "border-input",
            !digit && i === value.length && "border-ring ring-[3px] ring-ring/25",
          )}
        >
          {digit}
        </span>
      ))}
    </div>
  );
}
