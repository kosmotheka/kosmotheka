import { useTranslation } from "react-i18next";

import { Input } from "@/components/ui/input";

/**
 * The code of the second factor where an account is asked for it to do something delicate (D100): the 6 digits of the
 * app or, for whoever lost it, a backup code. One text field for both, since the two look nothing alike.
 */
export function SecondFactorInput({
  id,
  value,
  onChange,
  disabled,
  invalid,
}: {
  id?: string;
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  invalid?: boolean;
}) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-1.5">
      <Input
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        aria-invalid={invalid}
        inputMode="text"
        autoComplete="one-time-code"
        autoCapitalize="characters"
        autoCorrect="off"
        spellCheck={false}
        maxLength={16}
        placeholder="123456"
        className="font-mono tracking-wide"
      />
      <span className="text-[13px] text-muted-foreground">{t("account.confirm.appCodeHelp")}</span>
    </div>
  );
}
