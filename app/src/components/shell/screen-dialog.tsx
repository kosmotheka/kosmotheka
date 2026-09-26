import type { ComponentProps } from "react";

import { DialogContent } from "@/components/ui/dialog";
import { cn } from "@/lib/utils";

/** A dialog that takes the whole screen, the Android way to show a form or a detail page. */
export function ScreenDialogContent({ className, ...props }: ComponentProps<typeof DialogContent>) {
  return (
    <DialogContent
      showCloseButton={false}
      className={cn(
        "top-0 left-0 flex h-dvh max-h-none w-full max-w-none translate-x-0 translate-y-0 flex-col gap-0 overflow-hidden rounded-none p-0 pt-[env(safe-area-inset-top)] ring-0 sm:max-w-none",
        className,
      )}
      {...props}
    />
  );
}
