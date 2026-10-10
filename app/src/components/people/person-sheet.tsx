import { useState } from "react";
import { useTranslation } from "react-i18next";
import { EllipsisVertical, Info, Trash2, UserRoundCheck, UserRoundMinus } from "lucide-react";

import { Avatar, Switch, linkNote } from "@/components/people/role-parts";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { type Person, type PersonDevice, type Role, roles } from "@/lib/api";
import { cn } from "@/lib/utils";

/**
 * A person, on mobile (Design 02): their role here, whether they may share, and
 * their devices, each with a menu to remove it if it is lost.
 */
export function PersonSheet({
  person,
  library,
  onClose,
  onSave,
  onRemoveDevice,
  onWithdraw,
  onRestore,
}: {
  person: Person;
  library: string;
  onClose: () => void;
  onSave: (role: Role, canShare: boolean) => void;
  onRemoveDevice: (device: PersonDevice) => void;
  onWithdraw: () => void;
  onRestore: () => void;
}) {
  const { t } = useTranslation();
  const [role, setRole] = useState<Role>(person.role ?? "reader");
  const [canShare, setCanShare] = useState(person.canShare);
  const lastAdmin = person.lastAdmin;
  const isMe = person.devices.some((d) => d.thisDevice);

  if (person.accessWithdrawn) {
    // No role to change and no device to assign while they have no access (D98): only to give it back.
    return (
      <SheetOrDialog
        open
        desktop={false}
        onClose={onClose}
        title={person.name}
        description={t("people.noAccess")}
        icon={<Avatar name={person.name} size={48} />}
      >
        <p className="flex gap-2 text-[13px] leading-snug text-muted-foreground">
          <Info className="mt-0.5 size-4 shrink-0" />
          <span>{t("people.noAccess")}</span>
        </p>
        <p className="text-sm leading-relaxed text-pretty">
          {t("people.restore.then", { person: person.name })}
        </p>
        <div className="grid grid-cols-2 gap-2.5">
          <Button variant="outline" className="h-12" onClick={onClose}>
            {t("common.close")}
          </Button>
          <Button className="h-12" onClick={onRestore}>
            <UserRoundCheck /> {t("people.restore.menu")}
          </Button>
        </div>
      </SheetOrDialog>
    );
  }

  return (
    <SheetOrDialog
      open
      desktop={false}
      onClose={onClose}
      title={person.name}
      description={t("people.sheetDescription", { person: person.name })}
      icon={<Avatar name={person.name} size={48} />}
    >
      {person.elsewhere.length > 0 && (
        <p className="-mt-2 text-[13px] text-muted-foreground">
          {t("people.alsoIn", { libraries: person.elsewhere.join(", ") })}
        </p>
      )}
      <span className="-mb-2 text-[13px] font-medium text-muted-foreground">
        {t("people.roleIn", { library })}
      </span>
      <div role="radiogroup" className="flex flex-col divide-y overflow-hidden rounded-2xl border">
        {roles.map((option) => {
          const disabled = lastAdmin && option !== "admin";
          return (
            <button
              key={option}
              type="button"
              role="radio"
              aria-checked={option === role}
              disabled={disabled}
              onClick={() => setRole(option)}
              className={cn(
                "flex min-h-[60px] items-center gap-3.5 px-3.5 text-left disabled:opacity-50",
                option === role && "bg-primary/8",
              )}
            >
              <span
                className={cn(
                  "size-[22px] shrink-0 rounded-full border-[1.5px] border-muted-foreground",
                  option === role && "border-[6px] border-primary",
                )}
              />
              <span className="flex flex-col gap-0.5">
                <span className="text-[15px] font-medium">{t(`people.roles.${option}`)}</span>
                <span className="text-[13px] leading-snug text-muted-foreground">
                  {t(`people.roleHints.${option}`)}
                </span>
              </span>
            </button>
          );
        })}
      </div>
      {lastAdmin && (
        <p className="flex gap-2 text-[13px] leading-snug text-muted-foreground">
          <Info className="mt-0.5 size-4 shrink-0" />
          <span>
            {t("people.lastAdmin")}{" "}
            <strong className="font-semibold text-foreground">{t("people.lastAdminAction")}</strong>
          </span>
        </p>
      )}
      <div className="flex items-center gap-3.5">
        <span className="flex flex-1 flex-col gap-0.5">
          <span className="text-[15px] font-medium">{t("people.canShare")}</span>
          <span className="text-[13px] text-muted-foreground">
            {t("people.canShareHint", { library })}
          </span>
        </span>
        <Switch checked={canShare} label={t("people.canShare")} onChange={setCanShare} />
      </div>
      {person.devices.length > 0 && (
        <>
          <span className="-mb-2 text-[13px] font-medium text-muted-foreground">
            {t("people.theirDevices")}
          </span>
          <div className="flex flex-col">
            {person.devices.map((device) => (
              <div key={device.id} className="flex min-h-14 items-center gap-3">
                <span className="flex-1 text-[15px] font-medium">
                  {device.name}
                  {device.thisDevice && (
                    <span className="font-normal text-muted-foreground">
                      {" "}
                      · {t("people.thisDevice")}
                    </span>
                  )}
                  {linkNote(device, t) && (
                    <span className="block text-[13px] font-normal text-muted-foreground">
                      {linkNote(device, t)}
                    </span>
                  )}
                </span>
                {!device.thisDevice && (
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={t("people.deviceOptions", { device: device.name })}
                      >
                        <EllipsisVertical />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem
                        variant="destructive"
                        onSelect={() => onRemoveDevice(device)}
                      >
                        <Trash2 /> {t("people.revoke.menu")}
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                )}
              </div>
            ))}
          </div>
        </>
      )}
      {!isMe && (
        <Button
          variant="outline"
          className="h-12 text-destructive hover:text-destructive"
          onClick={onWithdraw}
        >
          <UserRoundMinus /> {t("people.kick.menu", { library })}
        </Button>
      )}
      <div className="grid grid-cols-2 gap-2.5">
        <Button variant="outline" className="h-12" onClick={onClose}>
          {t("common.cancel")}
        </Button>
        <Button className="h-12" onClick={() => onSave(role, canShare)}>
          {t("form.save")}
        </Button>
      </div>
    </SheetOrDialog>
  );
}
