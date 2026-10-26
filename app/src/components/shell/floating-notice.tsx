import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { CircleCheck, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const SHOWN_FOR_MS = 60_000;

/**
 * The app's notice, floating at the bottom so the page under it never moves. It goes by
 * itself after a minute, unless it carries an action the person may still want; the X
 * closes it at any time. Inverted colours, like Android's snackbar, read well in both themes.
 */
export function FloatingNotice({
  text,
  action,
  desktop,
  onClose,
}: {
  text: string;
  action?: { label: string; run: () => void };
  desktop: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const lasts = !action;

  useEffect(() => {
    if (!lasts) return;
    const timer = setTimeout(onClose, SHOWN_FOR_MS);
    return () => clearTimeout(timer);
  }, [text, lasts, onClose]);

  return (
    <div
      role="status"
      className={cn(
        "fixed z-40 flex items-start gap-3 rounded-xl bg-foreground py-2.5 pr-1.5 pl-4 text-sm text-background shadow-[0_8px_24px_-6px_oklch(0_0_0/40%)]",
        desktop
          ? "bottom-6 left-1/2 w-[min(36rem,calc(100vw-2rem))] -translate-x-1/2"
          : // Above the catalog's «Add» button, which sits above the bottom bar.
            "inset-x-4 bottom-[calc(180px+env(safe-area-inset-bottom))]",
      )}
    >
      <CircleCheck className="mt-0.5 size-4 shrink-0" />
      <div className="flex min-w-0 flex-1 flex-wrap items-center gap-x-3 gap-y-1 py-0.5 leading-snug">
        <span className="text-pretty">{text}</span>
        {action && (
          <Button
            variant="link"
            size="sm"
            className="h-auto p-0 text-background underline"
            onClick={action.run}
          >
            {action.label}
          </Button>
        )}
      </div>
      <Button
        variant="ghost"
        size="icon-sm"
        className="shrink-0 text-background hover:bg-background/15 hover:text-background"
        aria-label={t("common.close")}
        onClick={onClose}
      >
        <X />
      </Button>
    </div>
  );
}
