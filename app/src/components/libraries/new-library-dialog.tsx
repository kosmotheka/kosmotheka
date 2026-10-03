import { type SubmitEvent, useState } from "react";
import { useTranslation } from "react-i18next";
import { CircleAlert } from "lucide-react";

import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Sheet, SheetContent, SheetDescription, SheetTitle } from "@/components/ui/sheet";
import { createLibrary, isCommandError } from "@/lib/api";
import { closeOverlay, navigate, openOverlay } from "@/lib/navigation";

/** Only the name: the new library becomes the active one, empty. */
export function NewLibraryDialog({
  open,
  desktop,
  onCreated,
}: {
  open: boolean;
  desktop: boolean;
  onCreated: (name: string) => void;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const [name, setName] = useState("");
  const [error, setError] = useState<"required" | "out_of_range" | "name_in_use" | null>(null);
  const [saving, setSaving] = useState(false);

  const existing = libraries.all.find(
    (library) => nameOf(library).toLocaleLowerCase() === name.trim().toLocaleLowerCase(),
  );

  const close = () => {
    setName("");
    setError(null);
    closeOverlay();
  };

  const submit = async (event: SubmitEvent) => {
    event.preventDefault();
    setSaving(true);
    try {
      await createLibrary(name);
      libraries.reload();
      onCreated(name.trim());
      setName("");
      setError(null);
      navigate("catalog");
    } catch (e) {
      const code = isCommandError(e) && e.kind === "validation" ? e.errors[0]?.code : null;
      if (code === "required" || code === "out_of_range" || code === "name_in_use") setError(code);
      else throw e;
    } finally {
      setSaving(false);
    }
  };

  const openExisting = async () => {
    if (!existing) return;
    await libraries.switchTo(existing.id);
    close();
  };

  const form = (
    <form onSubmit={(e) => void submit(e)} noValidate className="flex flex-col gap-4">
      <p className="text-sm text-pretty text-muted-foreground">{t("libraries.newDescription")}</p>
      <div className="flex flex-col gap-2">
        <Label htmlFor="library-name">{t("libraries.name")}</Label>
        <Input
          id="library-name"
          autoFocus
          aria-invalid={error !== null}
          className={desktop ? undefined : "h-12 text-base"}
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            setError(null);
          }}
        />
        {error === "name_in_use" ? (
          <p role="alert" className="flex gap-2 text-[13px] leading-snug text-destructive">
            <CircleAlert className="mt-0.5 size-4 shrink-0" />
            <span className="flex flex-col gap-0.5">
              <span className="font-semibold">
                {t("libraries.nameInUse", { name: name.trim() })}
              </span>
              <span className="text-muted-foreground">{t("libraries.nameInUseHint")}</span>
            </span>
          </p>
        ) : error ? (
          <p role="alert" className="text-[13px] text-destructive">
            {t(`errors.${error}`)}
          </p>
        ) : (
          <p className="text-xs text-muted-foreground">{t("libraries.nameHint")}</p>
        )}
      </div>
      <div className="grid grid-cols-2 gap-2.5">
        {error === "name_in_use" && existing ? (
          <Button type="button" variant="outline" onClick={() => void openExisting()}>
            {t("libraries.openExisting", { name: nameOf(existing) })}
          </Button>
        ) : (
          <Button type="button" variant="outline" onClick={close}>
            {t("form.cancel")}
          </Button>
        )}
        <Button type="submit" disabled={saving || error === "name_in_use"}>
          {t(desktop ? "libraries.createAndOpen" : "libraries.create")}
        </Button>
      </div>
      <button
        type="button"
        className="self-start text-[13px] text-muted-foreground underline underline-offset-2"
        onClick={() => openOverlay({ name: "pair" })}
      >
        {t("libraries.joinInstead")}
      </button>
    </form>
  );

  const onOpenChange = (next: boolean) => !next && close();
  if (desktop) {
    return (
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="sm:max-w-md">
          <DialogTitle>{t("nav.newLibrary")}</DialogTitle>
          <DialogDescription className="sr-only">{t("libraries.newDescription")}</DialogDescription>
          {form}
        </DialogContent>
      </Dialog>
    );
  }
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        side="bottom"
        showCloseButton={false}
        className="gap-3 rounded-t-[28px] bg-popover px-6 pt-2.5 pb-[max(1.75rem,env(safe-area-inset-bottom))]"
      >
        <div className="mx-auto mb-1 h-1 w-8 rounded-full bg-muted-foreground/40" aria-hidden />
        <SheetTitle className="text-xl font-semibold">{t("nav.newLibrary")}</SheetTitle>
        <SheetDescription className="sr-only">{t("libraries.newDescription")}</SheetDescription>
        {form}
      </SheetContent>
    </Sheet>
  );
}
