import { useEffect } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { CircleCheck, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const SHOWN_FOR_MS = 60_000;

/** Its id: dialogs and sheets open under it do not take a tap on it for a tap outside them. */
const FLOATING_NOTICE_ID = "floating-notice";

/**
 * The app's notice, floating at the bottom so the page under it never moves. It goes by
 * itself after a minute, unless it carries an action the person may still want (that one
 * goes only after its own time, if it has one); the X closes it at any time. It sits over
 * any open dialog, such as a book's page, so what was done there is said there. Inverted
 * colours, like Android's snackbar, read well in both themes.
 */
export function FloatingNotice({
  text,
  action,
  desktop,
  onClose,
}: {
  text: string;
  action?: { label: string; run: () => void; closesAfterMs?: number };
  desktop: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const shownFor = action ? action.closesAfterMs : SHOWN_FOR_MS;

  useEffect(() => {
    if (shownFor === undefined) return;
    const timer = setTimeout(onClose, shownFor);
    return () => clearTimeout(timer);
  }, [text, shownFor, onClose]);

  // On the body, not in the app's root: an open dialog hides the root from screen readers.
  return createPortal(
    <div
      id={FLOATING_NOTICE_ID}
      role="status"
      className={cn(
        // Over dialogs (z-50), and reachable while one is open: a modal dialog turns off the rest of the page.
        "pointer-events-auto fixed z-[60] flex items-start gap-3 rounded-xl bg-foreground py-2.5 pr-1.5 pl-4 text-sm text-background shadow-[0_8px_24px_-6px_oklch(0_0_0/40%)]",
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
    </div>,
    document.body,
  );
}
