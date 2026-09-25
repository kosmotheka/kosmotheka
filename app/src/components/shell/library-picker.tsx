import { useTranslation } from "react-i18next";
import { Check, LibraryBig, Plus } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Sheet, SheetContent, SheetDescription, SheetTitle } from "@/components/ui/sheet";
import { closeOverlay } from "@/lib/navigation";

/**
 * "Your libraries": a bottom sheet on mobile, a dialog on desktop. Until
 * several libraries exist (roadmap phase 4) it lists the only one.
 */
export function LibraryPicker({
  open,
  desktop,
  library,
  count,
}: {
  open: boolean;
  desktop: boolean;
  library: string;
  count: number;
}) {
  const { t } = useTranslation();
  const onOpenChange = (next: boolean) => !next && closeOverlay();

  const body = (
    <>
      <button
        type="button"
        className="flex min-h-14 items-center gap-3.5 px-6 py-2.5 text-left hover:bg-muted"
        onClick={closeOverlay}
      >
        <span className="flex size-10 items-center justify-center rounded-xl bg-secondary">
          <LibraryBig className="size-5" />
        </span>
        <span className="flex flex-1 flex-col gap-0.5">
          <span className="text-[15px] font-medium">{library}</span>
          <span className="text-[13px] text-muted-foreground">{t("nav.works", { count })}</span>
        </span>
        <Check className="size-5" />
      </button>
      <div className="mx-6 my-2 h-px bg-border" />
      <div className="flex min-h-14 items-center gap-3.5 px-6 py-2.5 text-muted-foreground">
        <span className="flex size-10 items-center justify-center rounded-xl border border-dashed border-muted-foreground">
          <Plus className="size-5" />
        </span>
        <span className="flex flex-1 flex-col gap-0.5">
          <span className="text-[15px] font-medium">{t("nav.newLibrary")}</span>
          <span className="text-[13px]">{t("nav.newLibraryHint")}</span>
        </span>
        <Badge variant="outline" className="font-normal">
          {t("nav.soon")}
        </Badge>
      </div>
    </>
  );

  if (desktop) {
    return (
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="gap-0 px-0 pb-4 sm:max-w-md">
          <DialogTitle className="px-6 pb-2">{t("nav.libraries")}</DialogTitle>
          <DialogDescription className="sr-only">{t("nav.changeLibrary")}</DialogDescription>
          {body}
        </DialogContent>
      </Dialog>
    );
  }
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        side="bottom"
        showCloseButton={false}
        className="gap-0 rounded-t-[28px] bg-popover pt-2.5 pb-[max(1.75rem,env(safe-area-inset-bottom))]"
      >
        <div className="mx-auto mb-3.5 h-1 w-8 rounded-full bg-muted-foreground/40" aria-hidden />
        <SheetTitle className="px-6 pb-2 text-[17px] font-semibold">{t("nav.libraries")}</SheetTitle>
        <SheetDescription className="sr-only">{t("nav.changeLibrary")}</SheetDescription>
        {body}
      </SheetContent>
    </Sheet>
  );
}
