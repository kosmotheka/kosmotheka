import type { ReactNode } from "react";

import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Sheet, SheetContent, SheetDescription, SheetTitle } from "@/components/ui/sheet";
import { cn } from "@/lib/utils";

/** A bottom sheet on mobile, a dialog on desktop, for short confirmations and choices. */
export function SheetOrDialog({
  open,
  desktop,
  onClose,
  title,
  description,
  icon,
  className,
  children,
}: {
  open: boolean;
  desktop: boolean;
  onClose: () => void;
  title: string;
  /** For screen readers; the visible text goes in `children`. */
  description: string;
  icon?: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  const onOpenChange = (next: boolean) => !next && onClose();
  const heading = (
    <div className="flex items-center gap-3">
      {icon}
      <span className="text-xl leading-tight font-semibold tracking-tight">{title}</span>
    </div>
  );

  if (desktop) {
    return (
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className={cn("gap-4 sm:max-w-xl", className)}>
          <DialogTitle asChild>{heading}</DialogTitle>
          <DialogDescription className="sr-only">{description}</DialogDescription>
          {children}
        </DialogContent>
      </Dialog>
    );
  }
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        side="bottom"
        showCloseButton={false}
        className={cn(
          "gap-4 rounded-t-[28px] bg-popover px-6 pt-2.5 pb-[max(1.75rem,env(safe-area-inset-bottom))]",
          className,
        )}
      >
        <div className="mx-auto h-1 w-8 rounded-full bg-muted-foreground/40" aria-hidden />
        <SheetTitle asChild>{heading}</SheetTitle>
        <SheetDescription className="sr-only">{description}</SheetDescription>
        {children}
      </SheetContent>
    </Sheet>
  );
}
