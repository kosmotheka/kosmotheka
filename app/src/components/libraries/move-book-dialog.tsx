import { useState } from "react";
import { useTranslation } from "react-i18next";
import { FolderInput, Info } from "lucide-react";

import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { moveBook } from "@/lib/api";
import { closeOverlay } from "@/lib/navigation";
import { cn } from "@/lib/utils";

/** Moves a book of the active library to another one (D46). */
export function MoveBookDialog({
  open,
  desktop,
  editionId,
  title,
  onMoved,
}: {
  open: boolean;
  desktop: boolean;
  editionId: string;
  title: string;
  /** Called with the destination and the book's id there. */
  onMoved: (to: string, newEditionId: string) => void;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const others = libraries.all.filter((library) => !library.active);
  const [chosen, setChosen] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const target = others.find((library) => library.id === chosen) ?? others[0];
  const from = nameOf(libraries.active);

  if (!target) return null;

  const move = async () => {
    setBusy(true);
    setError(null);
    try {
      const newId = await moveBook(editionId, target.id);
      libraries.reload();
      onMoved(target.id, newId);
      closeOverlay();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={closeOverlay}
      title={t("libraries.moveTitle", { title })}
      description={t("libraries.moveNow", { name: from })}
      icon={<FolderInput className="size-5 shrink-0 text-muted-foreground" />}
    >
      <p className="-mt-2 text-sm text-muted-foreground">{t("libraries.moveNow", { name: from })}</p>
      <div role="radiogroup" className="-mx-2 flex flex-col">
        {others.map((library) => {
          const on = library.id === target.id;
          return (
            <button
              key={library.id}
              type="button"
              role="radio"
              aria-checked={on}
              className={cn(
                "flex min-h-14 items-center gap-4 rounded-xl px-2 text-left",
                on && "bg-accent",
              )}
              onClick={() => setChosen(library.id)}
            >
              <span
                className={cn(
                  "flex size-[22px] shrink-0 items-center justify-center rounded-full border-[1.5px]",
                  on ? "border-2 border-primary" : "border-muted-foreground",
                )}
              >
                {on && <span className="size-2.5 rounded-full bg-primary" />}
              </span>
              <span className="flex flex-col gap-0.5">
                <span className="text-[15px] font-medium">{nameOf(library)}</span>
                <span className="text-[13px] text-muted-foreground">
                  {t("nav.works", { count: library.works })}
                </span>
              </span>
            </button>
          );
        })}
      </div>
      <p className="flex gap-2.5 rounded-xl bg-muted px-3.5 py-3 text-sm leading-snug">
        <Info className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
        <span className="text-pretty">
          {t("libraries.moveExplained", { to: nameOf(target), from })}
        </span>
      </p>
      {error && <p className="text-[13px] text-destructive">{error}</p>}
      <div className="grid grid-cols-2 gap-2.5">
        <Button variant="outline" className="h-12" onClick={closeOverlay}>
          {t("form.cancel")}
        </Button>
        <Button className="h-12" disabled={busy} onClick={() => void move()}>
          {t("libraries.moveTo", { name: nameOf(target) })}
        </Button>
      </div>
    </SheetOrDialog>
  );
}
