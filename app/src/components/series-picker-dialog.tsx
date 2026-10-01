import { type FormEvent, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Layers, ListOrdered, Plus } from "lucide-react";

import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  type SeriesDetail,
  type SeriesSummary,
  addToSeries,
  createSeries,
  getSeries,
  listSeries,
} from "@/lib/api";
import { cn } from "@/lib/utils";

type Step = { kind: "search" } | { kind: "position"; series: SeriesSummary } | { kind: "new" };

/** «Añadir a una serie» from a work (Design's SeriesDialog): pick an existing series and its
 * number, or create one on the spot. */
export function SeriesPickerDialog({
  open,
  desktop,
  workId,
  workTitle,
  onClose,
  onAdded,
}: {
  open: boolean;
  desktop: boolean;
  workId: string;
  workTitle: string;
  onClose: () => void;
  onAdded: () => void;
}) {
  const { t } = useTranslation();
  const [all, setAll] = useState<SeriesSummary[]>([]);
  const [query, setQuery] = useState("");
  const [step, setStep] = useState<Step>({ kind: "search" });
  const [detail, setDetail] = useState<SeriesDetail | null>(null);
  const [position, setPosition] = useState("1");
  const [ordered, setOrdered] = useState(true);
  const [saving, setSaving] = useState(false);
  // Render-phase resets (the same trick `BookDetailView` uses for `shownId`) instead of
  // effects whose only job would be to mirror a prop or a previous state into this one.
  const [shownOpen, setShownOpen] = useState(open);
  const [shownStep, setShownStep] = useState(step);

  if (open !== shownOpen) {
    setShownOpen(open);
    if (open) {
      setQuery("");
      setStep({ kind: "search" });
    }
  }
  if (step !== shownStep) {
    setShownStep(step);
    if (step.kind !== "position" || !step.series.ordered) setDetail(null);
  }

  useEffect(() => {
    if (!open) return;
    listSeries()
      .then(setAll)
      .catch(() => setAll([]));
  }, [open]);

  useEffect(() => {
    if (step.kind !== "position" || !step.series.ordered) return;
    let cancelled = false;
    getSeries(step.series.id)
      .then((found) => {
        if (cancelled) return;
        setDetail(found);
        const max = Math.max(0, ...found.entries.map((e) => e.position ?? 0));
        setPosition(String(max + 1));
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [step]);

  const matches = all.filter((s) => s.name.toLowerCase().includes(query.trim().toLowerCase()));

  const pick = (series: SeriesSummary) => {
    if (series.ordered) setStep({ kind: "position", series });
    else void add(series.id, null);
  };

  const add = async (seriesId: string, pos: number | null) => {
    setSaving(true);
    try {
      await addToSeries(seriesId, workId, pos);
      onAdded();
    } finally {
      setSaving(false);
    }
  };

  const submitPosition = (event: FormEvent) => {
    event.preventDefault();
    if (step.kind !== "position") return;
    const n = Number(position);
    void add(step.series.id, Number.isFinite(n) && n > 0 ? n : null);
  };

  const submitNew = async (event: FormEvent) => {
    event.preventDefault();
    if (!query.trim()) return;
    setSaving(true);
    try {
      const id = await createSeries(query.trim(), ordered);
      await addToSeries(id, workId, ordered ? 1 : null);
      onAdded();
    } finally {
      setSaving(false);
    }
  };

  // A hint like «Between “Hijos de Dune” (3) and “Casa Capitular” (6)», simplified from
  // Design's exact "…and the gap at 5" wording (that needs computing gaps too).
  const positionHint = (() => {
    if (step.kind !== "position" || !detail) return null;
    const n = Number(position);
    if (!Number.isFinite(n)) return null;
    const before = detail.entries
      .filter((e) => e.position !== null && e.position < n)
      .sort((a, b) => (b.position ?? 0) - (a.position ?? 0))[0];
    const after = detail.entries
      .filter((e) => e.position !== null && e.position > n)
      .sort((a, b) => (a.position ?? 0) - (b.position ?? 0))[0];
    if (before && after) {
      return t("series.positionBetween", {
        prevTitle: before.title,
        prevPos: before.position,
        nextTitle: after.title,
        nextPos: after.position,
      });
    }
    if (before) return t("series.positionAfter", { title: before.title, pos: before.position });
    if (after) return t("series.positionBefore", { title: after.title, pos: after.position });
    return null;
  })();

  const body =
    step.kind === "search" ? (
      <div className="flex flex-col gap-3">
        <div className="space-y-2">
          <label htmlFor="series-picker-query" className="text-sm font-medium">
            {t("series.pickSeries")}
          </label>
          <Input
            id="series-picker-query"
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("series.picker.searchPlaceholder")}
          />
        </div>
        <div role="listbox" className="flex flex-col gap-1 rounded-lg border p-1">
          {query.trim() && (
            <button
              type="button"
              role="option"
              onClick={() => setStep({ kind: "new" })}
              className="flex h-10 items-center gap-2.5 rounded-md px-2.5 text-left text-sm hover:bg-muted"
            >
              <Plus className="size-[15px]" />
              <span className="flex-1">{t("series.createSeries", { name: query.trim() })}</span>
            </button>
          )}
          {matches.map((s) => (
            <button
              key={s.id}
              type="button"
              role="option"
              onClick={() => pick(s)}
              className="flex h-10 items-center gap-2.5 rounded-md px-2.5 text-left text-sm hover:bg-muted"
            >
              {s.ordered ? (
                <ListOrdered className="size-[15px]" />
              ) : (
                <Layers className="size-[15px]" />
              )}
              <span className="flex-1">{s.name}</span>
              <span className="text-xs text-muted-foreground">
                {t("series.list.workCount", { count: s.works })}
                {" · "}
                {t(s.ordered ? "series.ordered" : "series.unordered")}
              </span>
            </button>
          ))}
          {matches.length === 0 && !query.trim() && (
            <p className="px-2.5 py-2 text-sm text-muted-foreground">{t("series.empty")}</p>
          )}
        </div>
        <div className="flex justify-end">
          <Button type="button" variant="outline" onClick={onClose}>
            {t("form.cancel")}
          </Button>
        </div>
      </div>
    ) : step.kind === "position" ? (
      <form onSubmit={submitPosition} className="flex flex-col gap-4">
        <div className="flex items-end gap-3">
          <div className="space-y-2">
            <label htmlFor="series-picker-position" className="text-sm font-medium">
              {t("series.position")}
            </label>
            <Input
              id="series-picker-position"
              type="number"
              min={1}
              className="w-20 font-mono"
              value={position}
              onChange={(e) => setPosition(e.target.value)}
            />
          </div>
          {positionHint && (
            <p className="flex-1 pb-2.5 text-[13px] text-muted-foreground">{positionHint}</p>
          )}
        </div>
        <div className="flex justify-end gap-2">
          <Button type="button" variant="outline" onClick={() => setStep({ kind: "search" })}>
            {t("form.cancel")}
          </Button>
          <Button type="submit" disabled={saving}>
            {t("series.addToNamed", { name: step.series.name })}
          </Button>
        </div>
      </form>
    ) : (
      <form onSubmit={(e) => void submitNew(e)} className="flex flex-col gap-4">
        <div role="radiogroup" aria-label={t("series.type")} className="grid grid-cols-2 gap-2">
          {(
            [
              { key: true, icon: ListOrdered, label: "series.ordered", hint: "series.orderedHint" },
              { key: false, icon: Layers, label: "series.unordered", hint: "series.unorderedHint" },
            ] as const
          ).map((option) => (
            <button
              key={String(option.key)}
              type="button"
              role="radio"
              aria-checked={ordered === option.key}
              onClick={() => setOrdered(option.key)}
              className={cn(
                "flex flex-col gap-0.5 rounded-xl border p-3 text-left",
                ordered === option.key ? "border-primary ring-1 ring-primary" : "border-border",
              )}
            >
              <span className="flex items-center gap-1.5 text-sm font-semibold">
                <option.icon className="size-4" />
                {t(option.label)}
              </span>
              <span className="text-xs text-muted-foreground">{t(option.hint)}</span>
            </button>
          ))}
        </div>
        <div className="flex justify-end gap-2">
          <Button type="button" variant="outline" onClick={() => setStep({ kind: "search" })}>
            {t("form.cancel")}
          </Button>
          <Button type="submit" disabled={saving || !query.trim()}>
            {t("series.createAndAdd")}
          </Button>
        </div>
      </form>
    );

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={onClose}
      title={t("series.addToSeries")}
      description={workTitle}
    >
      <p className="-mt-2 text-sm text-muted-foreground">{workTitle}</p>
      {body}
    </SheetOrDialog>
  );
}
