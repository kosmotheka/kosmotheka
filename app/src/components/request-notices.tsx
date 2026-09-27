import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Inbox } from "lucide-react";

import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { type ChangeRequest, dismissRequestNotices, requestNotices } from "@/lib/api";
import { navigate } from "@/lib/navigation";
import { cn } from "@/lib/utils";
import { capitalize, describe } from "@/pages/requests-page";

/**
 * «A change of yours became a request» (Design, phase 5 §5, D73): after a sync
 * recalculated the library, what this device had done and may no longer do.
 */
export function RequestNotices({ version, desktop }: { version: number; desktop: boolean }) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const library = useLibraryName()(libraries.active);
  const [notices, setNotices] = useState<ChangeRequest[]>([]);

  useEffect(() => {
    let cancelled = false;
    requestNotices()
      .then((loaded) => !cancelled && setNotices(loaded ?? []))
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [version]);

  if (notices.length === 0) return null;

  const close = (then?: () => void) => {
    setNotices([]);
    void dismissRequestNotices().catch(() => {});
    then?.();
  };

  return (
    <SheetOrDialog
      open
      desktop={desktop}
      onClose={() => close()}
      title={t("notices.title", { count: notices.length })}
      description={t("notices.why", { library })}
      icon={
        <span className="flex size-10 shrink-0 items-center justify-center rounded-full bg-warning-soft text-warning">
          <Inbox className="size-5" />
        </span>
      }
    >
      <ul className="flex flex-col divide-y rounded-xl border">
        {notices.map((notice) => (
          <li key={notice.id} className="px-3.5 py-3 text-sm font-medium">
            {capitalize(describe(t, notice))}
          </li>
        ))}
      </ul>
      <div className="flex flex-col gap-2 text-sm leading-relaxed text-pretty">
        <p>{t("notices.why", { library })}</p>
        <p className="text-[13px] text-muted-foreground">{t("notices.notLost")}</p>
      </div>
      <div className={cn("flex gap-2", desktop ? "justify-end" : "flex-col-reverse")}>
        <Button variant="outline" className={cn(!desktop && "h-12")} onClick={() => close()}>
          {t("requests.understood")}
        </Button>
        <Button
          className={cn(!desktop && "h-12")}
          onClick={() => close(() => navigate("requests"))}
        >
          {t("notices.see", { count: notices.length })}
        </Button>
      </div>
    </SheetOrDialog>
  );
}
