import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ShieldCheck, UserRoundCheck, UserRoundMinus } from "lucide-react";

import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { type Person, permissionErrorKey, restorePerson, withdrawPerson } from "@/lib/api";
import { cn } from "@/lib/utils";

/**
 * Sending a person away from the library, or giving the access back (D98, Design `kick`,
 * `kickLast` and `people.restore`). When Rust says they are the last administrator who can
 * act, the dialog only explains it: nothing is attempted and then refused.
 */
export function AccessDialog({
  person,
  mode,
  library,
  desktop,
  onClose,
  onDone,
}: {
  person: Person;
  mode: "withdraw" | "restore";
  library: string;
  desktop: boolean;
  onClose: () => void;
  onDone: (message: string) => void;
}) {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const blocked = mode === "withdraw" && person.lastAdmin;
  const vars = { person: person.name, library };

  const confirm = async () => {
    setBusy(true);
    try {
      await (mode === "withdraw" ? withdrawPerson(person.id) : restorePerson(person.id));
      onDone(t(mode === "withdraw" ? "people.kick.done" : "people.restore.done", vars));
    } catch (e) {
      const key = permissionErrorKey(e);
      onDone(key ? t(key) : String(e));
    } finally {
      setBusy(false);
    }
  };

  const Icon = blocked ? ShieldCheck : mode === "withdraw" ? UserRoundMinus : UserRoundCheck;
  const title = blocked
    ? t("people.kick.lastTitle", vars)
    : t(mode === "withdraw" ? "people.kick.title" : "people.restore.title", vars);
  const buttons = cn("flex gap-2", desktop ? "justify-end" : "flex-col-reverse");
  const size = cn(!desktop && "h-12");

  return (
    <SheetOrDialog
      open
      desktop={desktop}
      onClose={onClose}
      title={title}
      description={title}
      icon={
        <span
          className={cn(
            "flex size-10 shrink-0 items-center justify-center rounded-full",
            mode === "withdraw" && !blocked
              ? "bg-destructive/10 text-destructive"
              : "bg-muted text-muted-foreground",
          )}
        >
          <Icon className="size-5" />
        </span>
      }
    >
      <div className="flex flex-col gap-2 text-sm leading-relaxed text-pretty">
        {blocked ? (
          <>
            <p>{t("people.kick.lastBody", vars)}</p>
            <p className="text-[13px] text-muted-foreground">{t("people.kick.lastNote")}</p>
          </>
        ) : mode === "withdraw" ? (
          <>
            <p>{t("people.kick.body", vars)}</p>
            <p className="text-[13px] text-muted-foreground">{t("people.kick.keeps", vars)}</p>
            <p className="text-[13px] text-muted-foreground">{t("people.kick.foot")}</p>
          </>
        ) : (
          <>
            <p>{t("people.restore.body", vars)}</p>
            <p className="text-[13px] text-muted-foreground">{t("people.restore.then", vars)}</p>
          </>
        )}
      </div>
      <div className={buttons}>
        <Button variant="outline" className={size} onClick={onClose}>
          {blocked ? t("common.close") : t("common.cancel")}
        </Button>
        {!blocked && (
          <Button
            variant={mode === "withdraw" ? "destructive" : "default"}
            className={size}
            disabled={busy}
            onClick={() => void confirm()}
          >
            {t(mode === "withdraw" ? "people.kick.confirm" : "people.restore.confirm", vars)}
          </Button>
        )}
      </div>
    </SheetOrDialog>
  );
}
