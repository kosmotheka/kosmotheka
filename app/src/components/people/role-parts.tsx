import { useTranslation } from "react-i18next";
import { Check, ChevronDown, Info, Laptop, Smartphone } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { type PersonDevice, type Role, roles } from "@/lib/api";
import { cn } from "@/lib/utils";

/** The initial of a name, in a circle. */
export function Avatar({ name, size = 32 }: { name: string; size?: number }) {
  return (
    <span
      aria-hidden
      className="flex shrink-0 items-center justify-center rounded-full bg-secondary font-semibold text-secondary-foreground"
      style={{ width: size, height: size, fontSize: Math.round(size * 0.4) }}
    >
      {name.trim().charAt(0).toUpperCase() || "?"}
    </span>
  );
}

/** A device's name with an icon: a laptop for desktops, a phone otherwise. */
export function DeviceChip({ device }: { device: PersonDevice }) {
  const { t } = useTranslation();
  const Icon = /port|lap|desk|pc|mac/i.test(device.name) ? Laptop : Smartphone;
  return (
    <span className="inline-flex h-[26px] items-center gap-1.5 rounded-md bg-secondary px-2 text-[12.5px] whitespace-nowrap">
      <Icon className="size-3.5" />
      {device.name}
      {device.thisDevice && (
        <span className="text-muted-foreground">· {t("people.thisDevice")}</span>
      )}
    </span>
  );
}

/** A switch, as in the Design screens: on is the brand color. */
export function Switch({
  checked,
  disabled,
  label,
  onChange,
}: {
  checked: boolean;
  disabled?: boolean;
  label: string;
  onChange?: (checked: boolean) => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange?.(!checked)}
      className={cn(
        "relative block h-[21px] w-9 shrink-0 rounded-full transition-colors disabled:opacity-50",
        checked ? "bg-primary" : "bg-muted-foreground/45",
      )}
    >
      <span
        className={cn(
          "absolute top-[3px] size-[15px] rounded-full transition-all",
          checked ? "left-[18px] bg-primary-foreground" : "left-[3px] bg-background",
        )}
      />
    </button>
  );
}

/**
 * The role of a person, and a menu to change it where each option says what it
 * allows. The only administrator cannot pick anything else (D65).
 */
export function RoleMenu({
  role,
  lastAdmin,
  disabled,
  onChange,
  className,
}: {
  role: Role | null;
  lastAdmin: boolean;
  disabled?: boolean;
  onChange: (role: Role) => void;
  className?: string;
}) {
  const { t } = useTranslation();
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          disabled={disabled}
          className={cn("w-[180px] justify-between font-normal", className)}
        >
          {role ? t(`people.roles.${role}`) : t("people.noRole")}
          <ChevronDown className="text-muted-foreground" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-80">
        {roles.map((option) => (
          <DropdownMenuItem
            key={option}
            disabled={lastAdmin && option !== "admin"}
            className="items-start gap-2.5 py-2"
            onSelect={() => option !== role && onChange(option)}
          >
            <span className="mt-0.5 w-4 shrink-0 text-primary">
              {option === role && <Check className="size-4" />}
            </span>
            <span className="flex flex-col gap-0.5">
              <span className="font-medium">{t(`people.roles.${option}`)}</span>
              <span className="text-[12.5px] leading-snug text-muted-foreground">
                {t(`people.roleHints.${option}`)}
              </span>
            </span>
          </DropdownMenuItem>
        ))}
        {lastAdmin && (
          <>
            <DropdownMenuSeparator />
            <p className="flex gap-2.5 px-2.5 py-2 text-[12.5px] leading-snug text-muted-foreground">
              <Info className="mt-0.5 size-4 shrink-0" />
              <span>
                {t("people.lastAdmin")}{" "}
                <strong className="font-semibold text-foreground">
                  {t("people.lastAdminAction")}
                </strong>
              </span>
            </p>
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** What each role may do, under the table. */
export function RoleLegend({ compact = false }: { compact?: boolean }) {
  const { t } = useTranslation();
  if (compact) {
    return <p className="text-center text-xs text-muted-foreground">{t("people.legendShort")}</p>;
  }
  return (
    <div className="flex flex-wrap gap-x-5 gap-y-1 text-[12.5px] text-muted-foreground">
      {roles.map((role) => (
        <span key={role}>
          <strong className="font-semibold text-foreground">{t(`people.roles.${role}`)}:</strong>{" "}
          {t(`people.roleLegend.${role}`)}
        </span>
      ))}
    </div>
  );
}
