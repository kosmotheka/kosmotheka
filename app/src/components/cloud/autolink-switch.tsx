import { useState } from "react";
import { useTranslation } from "react-i18next";
import { CloudCog } from "lucide-react";

import { Switch } from "@/components/people/role-parts";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import {
  type LibraryCloudStatus,
  cloudErrorKey,
  permissionErrorKey,
  setCloudPower,
} from "@/lib/api";
import { refreshCloudStatus } from "@/lib/cloud";
import { cn } from "@/lib/utils";

/**
 * «The cloud can give access to new devices» (Design `autolink`, D76): in the settings of a library that is
 * in the cloud. Whoever administers it changes it, and turning it off asks first; the others see the state.
 * Rust decides who may (`status.administers`) and what the state is (`status.cloudPower`).
 */
export function AutolinkSwitch({
  status,
  library,
  desktop,
}: {
  status: LibraryCloudStatus;
  library: string;
  desktop: boolean;
}) {
  const { t } = useTranslation();
  const [asking, setAsking] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const on = status.cloudPower;

  const change = async (next: boolean) => {
    setBusy(true);
    setError(null);
    try {
      await setCloudPower(status.library, next);
    } catch (e) {
      const permission = permissionErrorKey(e);
      setError(t(permission ?? cloudErrorKey(e)));
    } finally {
      setBusy(false);
      setAsking(false);
      await refreshCloudStatus();
    }
  };

  return (
    <div className="flex flex-col gap-2 rounded-2xl border px-3.5 py-3">
      <div className="flex items-center gap-3.5">
        <span className="flex min-w-0 flex-1 flex-col gap-0.5">
          <span className="text-[15px] font-medium">{t("cloud.autolink.label")}</span>
          <span className="text-[13px] text-pretty text-muted-foreground">
            {t(on ? "cloud.autolink.on" : "cloud.autolink.off")}
          </span>
          {!status.administers && (
            <span className="text-[13px] text-muted-foreground">
              {t("cloud.autolink.readonly", { library })}
            </span>
          )}
        </span>
        {status.administers && (
          <Switch
            checked={on}
            disabled={busy}
            label={t("cloud.autolink.label")}
            onChange={(next) => (next ? void change(true) : setAsking(true))}
          />
        )}
      </div>
      {error && <p className="text-[13px] text-destructive">{error}</p>}
      {asking && (
        <SheetOrDialog
          open
          desktop={desktop}
          onClose={() => setAsking(false)}
          title={t("cloud.autolink.offTitle", { library })}
          description={t("cloud.autolink.offBody")}
          icon={
            <span className="flex size-10 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground">
              <CloudCog className="size-5" />
            </span>
          }
        >
          <p className="text-sm leading-relaxed text-pretty">{t("cloud.autolink.offBody")}</p>
          <div className={cn("flex gap-2", desktop ? "justify-end" : "flex-col-reverse")}>
            <Button
              variant="outline"
              className={cn(!desktop && "h-12")}
              onClick={() => setAsking(false)}
            >
              {t("common.cancel")}
            </Button>
            <Button
              variant="destructive"
              className={cn(!desktop && "h-12")}
              disabled={busy}
              onClick={() => void change(false)}
            >
              {t("cloud.autolink.offConfirm")}
            </Button>
          </div>
        </SheetOrDialog>
      )}
    </div>
  );
}
