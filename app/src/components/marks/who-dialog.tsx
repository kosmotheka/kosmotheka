import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Info } from "lucide-react";

import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { Avatar } from "@/components/people/role-parts";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { type ClaimCandidate, claimCandidates, claimPerson, permissionErrorKey } from "@/lib/api";
import { cn } from "@/lib/utils";

/**
 * «Who are you?» (Design, phase 5 §5.8): in a library without roles, the first
 * time this device marks something it says which person it belongs to. It does
 * not turn roles on.
 */
export function WhoDialog({
  desktop,
  onClose,
  onClaimed,
}: {
  desktop: boolean;
  onClose: () => void;
  onClaimed: () => void | Promise<void>;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const library = useLibraryName()(libraries.active);
  const [candidates, setCandidates] = useState<ClaimCandidate[]>([]);
  const [pick, setPick] = useState<string>("new");
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    claimCandidates()
      .then((loaded) => {
        setCandidates(loaded);
        const [firstCandidate] = loaded;
        if (firstCandidate) setPick(firstCandidate.id);
      })
      .catch(() => setCandidates([]));
  }, []);

  const chosen = candidates.find((c) => c.id === pick);
  const who = chosen?.name ?? name.trim();

  const confirm = async () => {
    setBusy(true);
    setError(null);
    try {
      await claimPerson({ id: chosen?.id ?? null, name: who });
      await onClaimed();
    } catch (e) {
      const key = permissionErrorKey(e);
      setError(key ? t(key) : String(e));
    } finally {
      setBusy(false);
    }
  };

  const radio = (on: boolean) => (
    <span
      className={cn(
        "flex size-[18px] shrink-0 items-center justify-center rounded-full border-[1.5px] border-muted-foreground",
        on && "border-2 border-primary",
      )}
    >
      {on && <span className="size-2 rounded-full bg-primary" />}
    </span>
  );

  return (
    <SheetOrDialog
      open
      desktop={desktop}
      onClose={onClose}
      title={t("marks.who.title")}
      description={t(desktop ? "marks.who.intro" : "marks.who.introPhone", { library })}
    >
      <p className="-mt-2 text-sm leading-normal text-pretty text-muted-foreground">
        {t(desktop ? "marks.who.intro" : "marks.who.introPhone", { library })}
      </p>
      <div role="radiogroup" className="flex flex-col divide-y overflow-hidden rounded-xl border">
        {candidates.map((c) => (
          <button
            key={c.id}
            type="button"
            role="radio"
            aria-checked={pick === c.id}
            onClick={() => setPick(c.id)}
            className={cn(
              "flex min-h-[52px] items-center gap-3 px-3.5 text-left",
              pick === c.id && "bg-primary/8",
            )}
          >
            {radio(pick === c.id)}
            <Avatar name={c.name} size={28} />
            <span className="flex min-w-0 flex-1 flex-col gap-px">
              <span className="text-sm font-medium">{c.name}</span>
              <span className="text-xs text-muted-foreground">
                {t("marks.who.copies", { count: c.copies, library })}
              </span>
            </span>
          </button>
        ))}
        <div
          role="radio"
          aria-checked={pick === "new"}
          tabIndex={-1}
          onClick={() => setPick("new")}
          className={cn(
            "flex min-h-[52px] items-center gap-3 px-3.5 py-2",
            pick === "new" && "bg-primary/8",
          )}
        >
          {radio(pick === "new")}
          <Input
            aria-label={t("marks.who.someoneElse")}
            placeholder={t("marks.who.someoneElse")}
            value={name}
            onChange={(e) => {
              setName(e.target.value);
              setPick("new");
            }}
          />
        </div>
      </div>
      <p className="flex items-start gap-2 text-[12.5px] leading-snug text-pretty text-muted-foreground">
        <Info className="mt-0.5 size-4 shrink-0" />
        {t("marks.who.noRoles", { library })}
      </p>
      {error && <p className="text-sm text-destructive">{error}</p>}
      <div className={cn("flex gap-2", desktop ? "justify-end" : "flex-col-reverse")}>
        {desktop && (
          <Button variant="outline" onClick={onClose}>
            {t("marks.who.notNow")}
          </Button>
        )}
        <Button
          className={cn(!desktop && "h-12")}
          disabled={!who || busy}
          onClick={() => void confirm()}
        >
          {who
            ? t(desktop ? "marks.who.iAm" : "marks.who.continueAs", { person: who })
            : t("marks.who.continue")}
        </Button>
      </div>
    </SheetOrDialog>
  );
}
