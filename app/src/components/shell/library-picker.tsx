import { useTranslation } from "react-i18next";
import { Check, LibraryBig, Plus, Settings } from "lucide-react";

import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Sheet, SheetContent, SheetDescription, SheetTitle } from "@/components/ui/sheet";
import { closeOverlay, navigate, openOverlay } from "@/lib/navigation";

/** "Your libraries": a bottom sheet on mobile, a dialog on desktop. */
export function LibraryPicker({ open, desktop }: { open: boolean; desktop: boolean }) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const onOpenChange = (next: boolean) => !next && closeOverlay();

  const choose = async (id: string) => {
    if (id !== libraries.active?.id) await libraries.switchTo(id);
    navigate("catalog");
  };
  const openSettings = async (id: string) => {
    if (id !== libraries.active?.id) await libraries.switchTo(id);
    navigate("library");
  };

  const body = (
    <>
      {libraries.all.map((library) => (
        <div key={library.id} className="flex min-h-16 items-center gap-1 pr-3 pl-2">
          <button
            type="button"
            aria-current={library.active || undefined}
            className="flex flex-1 items-center gap-3.5 rounded-xl px-4 py-2 text-left hover:bg-muted"
            onClick={() => void choose(library.id)}
          >
            <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-secondary">
              <LibraryBig className="size-5" />
            </span>
            <span className="flex min-w-0 flex-1 flex-col gap-0.5">
              <span className="flex items-center gap-1.5 text-[15px] font-medium">
                <span className="truncate">{nameOf(library)}</span>
                {library.active && <Check className="size-4 shrink-0" strokeWidth={2.5} />}
              </span>
              <span className="text-[13px] text-muted-foreground">
                {t("nav.works", { count: library.works })}
              </span>
            </span>
          </button>
          <Button
            variant="ghost"
            size="icon-lg"
            className="rounded-full text-muted-foreground"
            aria-label={t("libraries.settingsOf", { name: nameOf(library) })}
            onClick={() => void openSettings(library.id)}
          >
            <Settings />
          </Button>
        </div>
      ))}
      <div className="mx-6 my-2 h-px bg-border" />
      <button
        type="button"
        className="flex min-h-14 items-center gap-3.5 px-6 py-2.5 text-left hover:bg-muted"
        onClick={() => openOverlay({ name: "newLibrary" })}
      >
        <span className="flex size-10 items-center justify-center rounded-xl border border-dashed border-muted-foreground">
          <Plus className="size-5" />
        </span>
        <span className="flex flex-1 flex-col gap-0.5">
          <span className="text-[15px] font-medium">{t("nav.newLibrary")}</span>
          <span className="text-[13px] text-muted-foreground">{t("nav.newLibraryHint")}</span>
        </span>
      </button>
    </>
  );

  const count = t("libraries.onThisDevice", { count: libraries.all.length });
  if (desktop) {
    return (
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="gap-0 px-0 pb-4 sm:max-w-md">
          <div className="flex items-baseline justify-between px-6 pb-2">
            <DialogTitle>{t("nav.libraries")}</DialogTitle>
            <span className="text-[13px] text-muted-foreground">{count}</span>
          </div>
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
        <div className="flex items-baseline justify-between px-6 pb-2">
          <SheetTitle className="text-[17px] font-semibold">{t("nav.libraries")}</SheetTitle>
          <span className="text-[13px] text-muted-foreground">{count}</span>
        </div>
        <SheetDescription className="sr-only">{t("nav.changeLibrary")}</SheetDescription>
        {body}
      </SheetContent>
    </Sheet>
  );
}
