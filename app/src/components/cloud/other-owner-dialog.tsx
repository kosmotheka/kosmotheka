import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { UserRound } from "lucide-react";

import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { type OtherOwner, answerOtherOwner, cloudOtherOwners } from "@/lib/api";
import { useCloud } from "@/lib/cloud";
import { cn } from "@/lib/utils";

/**
 * «In Casa, this device belongs to Hugo» (Design `Dialog`, D76): the account signed in is another person
 * of the library. The app cannot change whose the device is (only who administers can, and Rust does not
 * know which person the account is), so the only answer is to leave it as it is. Asked once per account and
 * person; Rust remembers the answer.
 */
export function OtherOwnerDialog({ desktop }: { desktop: boolean }) {
  const { t } = useTranslation();
  const cloud = useCloud();
  const [open, setOpen] = useState<OtherOwner[]>([]);

  // After each sync: that is when a claim can say it.
  useEffect(() => {
    if (cloud.syncing) return;
    let cancelled = false;
    void cloudOtherOwners()
      .then((owners) => !cancelled && setOpen(owners))
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [cloud.syncing]);

  const current = open[0];
  if (!current) return null;

  const keep = async () => {
    await answerOtherOwner(current.library).catch(() => {});
    setOpen((owners) => owners.filter((o) => o.library !== current.library));
  };
  const role = current.role && t(`people.roles.${current.role}`).toLocaleLowerCase();

  return (
    <SheetOrDialog
      open
      desktop={desktop}
      onClose={() => void keep()}
      title={t("cloud.otherOwner.title", { library: current.libraryName, person: current.person })}
      description={t("cloud.otherOwner.body")}
      icon={
        <span className="flex size-10 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground">
          <UserRound className="size-5" />
        </span>
      }
    >
      <div className="flex flex-col gap-2 text-sm leading-relaxed text-pretty">
        <p>{t("cloud.otherOwner.body")}</p>
        <p>
          {t("cloud.otherOwner.keep", {
            library: current.libraryName,
            as: [current.person, role].filter(Boolean).join(" · "),
          })}
        </p>
        <p className="text-muted-foreground">{t("cloud.otherOwner.change")}</p>
      </div>
      <div className={cn("flex gap-2", desktop ? "justify-end" : "flex-col")}>
        <Button className={cn(!desktop && "h-12")} onClick={() => void keep()}>
          {t("cloud.otherOwner.action")}
        </Button>
      </div>
      <p className="text-[13px] text-muted-foreground">{t("cloud.otherOwner.foot")}</p>
    </SheetOrDialog>
  );
}
