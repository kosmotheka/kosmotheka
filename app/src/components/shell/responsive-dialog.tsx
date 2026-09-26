import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { X } from "lucide-react";

import { ScreenDialogContent } from "@/components/shell/screen-dialog";
import { useDesktop } from "@/components/shell/use-desktop";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";

/** A dialog on desktop, a full-screen page with a close button on mobile. */
export function ResponsiveDialog({
  open,
  onOpenChange,
  title,
  description,
  className,
  children,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  /** Desktop dialog classes, such as its width. */
  className?: string;
  children: ReactNode;
}) {
  const { t } = useTranslation();
  const desktop = useDesktop();

  if (desktop) {
    return (
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className={cn("flex max-h-[90vh] flex-col overflow-y-auto", className)}>
          <DialogHeader>
            <DialogTitle>{title}</DialogTitle>
            <DialogDescription className={description ? undefined : "sr-only"}>
              {description ?? title}
            </DialogDescription>
          </DialogHeader>
          {children}
        </DialogContent>
      </Dialog>
    );
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <ScreenDialogContent>
        <header className="flex h-16 shrink-0 items-center gap-1 px-1">
          <Button
            variant="ghost"
            size="icon-lg"
            className="size-12 rounded-full"
            aria-label={t("common.close")}
            onClick={() => onOpenChange(false)}
          >
            <X className="size-[22px]" />
          </Button>
          <DialogTitle className="text-lg font-semibold">{title}</DialogTitle>
        </header>
        <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 pt-1 pb-10">
          <DialogDescription className={description ? undefined : "sr-only"}>
            {description ?? title}
          </DialogDescription>
          {children}
        </div>
      </ScreenDialogContent>
    </Dialog>
  );
}
