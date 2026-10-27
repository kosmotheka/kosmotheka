import { type PointerEvent, useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ArrowLeft,
  Check,
  GripVertical,
  ListChecks,
  Pencil,
  Plus,
  Trash2,
  TriangleAlert,
  X,
} from "lucide-react";

import { AddToSeriesWorkDialog } from "@/components/series-work-picker";
import { ConfirmButton } from "@/components/confirm-button";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { ScreenDialogContent } from "@/components/shell/screen-dialog";
import { Button } from "@/components/ui/button";
import { Cover } from "@/components/cover";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  type SeriesDetail,
  type SeriesEntry,
  deleteSeries,
  getSeries,
  removeFromSeries,
  reorderSeries,
  updateSeries,
} from "@/lib/api";
import { navigate, openOverlay } from "@/lib/navigation";
import { cn } from "@/lib/utils";

type Row = { position: number | null; entry: SeriesEntry | null };

/**
 * Every number from 1 to the highest position, a missing one becoming a gap row (D61). No work is ever hidden: two on
 * one number (from before numbers made room, or from two devices) both show, and one with no number comes last.
 */
function rowsOf(detail: SeriesDetail): Row[] {
  if (!detail.ordered) return detail.entries.map((entry) => ({ position: null, entry }));
  const max = Math.max(0, ...detail.entries.map((e) => e.position ?? 0));
  const numbered = Array.from({ length: max }, (_, i): Row[] => {
    const position = i + 1;
    const here = detail.entries.filter((e) => e.position === position);
    return here.length > 0
      ? here.map((entry) => ({ position, entry }))
      : [{ position, entry: null }];
  }).flat();
  const unnumbered = detail.entries
    .filter((e) => e.position === null)
    .map((entry) => ({ position: null, entry }));
  return [...numbered, ...unnumbered];
}

/** Whether a drag moves the row at `index`: the rows between where it was taken and where it lands. */
function shifted(drag: { from: number; to: number }, index: number): boolean {
  return index >= Math.min(drag.from, drag.to) && index <= Math.max(drag.from, drag.to);
}

/** The number a row stands for, as the number field writes it. */
function numberOf(row: Row): string {
  return row.position === null ? "" : String(row.position);
}

/**
 * Moves what is in the row at `from` to `to`, as in a list: the numbers stay where they are and the works and gaps
 * under them shift, so dragging volume 5 onto 2 makes it 2 and moves 2, 3 and 4 one number down.
 */
function moveRow(rows: Row[], from: number, to: number): Row[] {
  const contents = rows.map((row) => row.entry);
  const [moved] = contents.splice(from, 1);
  contents.splice(to, 0, moved ?? null);
  return rows.map((row, i) => ({ position: row.position, entry: contents[i] ?? null }));
}

/**
 * A series' page (Design's SeriesPage): full screen on mobile, a wide dialog on desktop
 * standing in for the main-area page Design drew (D61).
 */
export function SeriesDetailView({
  seriesId,
  desktop,
  onClose,
  onChanged,
}: {
  seriesId: string | null;
  desktop: boolean;
  onClose: () => void;
  onChanged: () => void;
}) {
  const { t } = useTranslation();
  const [detail, setDetail] = useState<SeriesDetail | null>(null);
  const [reorder, setReorder] = useState(false);
  const [positions, setPositions] = useState<Record<string, string>>({});
  const [positionError, setPositionError] = useState<string | null>(null);
  // While editing an ordered series: the rows in the order the drags left them. Typing a number never moves a row,
  // so the list does not jump under the finger; the numbers decide when it is saved.
  const [editRows, setEditRows] = useState<Row[]>([]);
  // A drag in progress: the row taken, where it would land, and the middle of every row when it started.
  const [drag, setDrag] = useState<{ from: number; to: number; middles: number[] } | null>(null);
  const rowElements = useRef<(HTMLDivElement | null)[]>([]);
  const [renaming, setRenaming] = useState(false);
  const [addingWork, setAddingWork] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  // Another series opened: leave reorder mode the same render-phase way `BookDetailView`
  // resets its own `shownId` prop mirror, instead of an effect just to call `setReorder`.
  const [shownSeriesId, setShownSeriesId] = useState(seriesId);
  if (shownSeriesId !== seriesId) {
    setShownSeriesId(seriesId);
    setReorder(false);
  }

  const reload = useCallback(() => {
    if (!seriesId) return;
    getSeries(seriesId)
      .then(setDetail)
      .catch(() => setDetail(null));
  }, [seriesId]);

  useEffect(reload, [reload]);

  const shown = detail?.id === seriesId ? detail : null;
  const rows = shown ? rowsOf(shown) : [];
  const draggable = reorder && (shown?.ordered ?? false);
  // While dragging, the list shows where the row would land.
  const shownRows = draggable ? (drag ? moveRow(editRows, drag.from, drag.to) : editRows) : rows;
  const missing = shown ? shown.entries.filter((e) => e.copies === 0).length : 0;
  const gaps = rows.filter((r) => r.entry === null).length;

  const summary =
    shown &&
    [
      t(shown.ordered ? "series.orderedKind" : "series.unorderedKind"),
      t("series.haveOf", { have: shown.have, total: shown.total }),
      shown.ordered && missing > 0 ? t("series.missing", { count: missing }) : null,
      shown.ordered && gaps > 0 ? t("series.gaps", { count: gaps }) : null,
    ]
      .filter(Boolean)
      .join(" · ");

  const changed = () => {
    reload();
    onChanged();
  };

  const startReorder = () => {
    if (!shown) return;
    const map: Record<string, string> = {};
    for (const entry of shown.entries) map[entry.entryId] = String(entry.position ?? "");
    setPositions(map);
    setEditRows(rowsOf(shown));
    setDrag(null);
    setReorder(true);
  };

  const startDrag = (event: PointerEvent<HTMLButtonElement>, from: number) => {
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    const middles = rowElements.current.slice(0, editRows.length).map((element) => {
      const box = element?.getBoundingClientRect();
      return box ? box.top + box.height / 2 : 0;
    });
    setDrag({ from, to: from, middles });
  };

  const moveDrag = (event: PointerEvent<HTMLButtonElement>) => {
    if (!drag) return;
    // Where it lands: after every other row whose middle it has passed.
    const to = drag.middles.filter((middle, i) => i !== drag.from && middle < event.clientY).length;
    if (to !== drag.to) setDrag({ ...drag, to });
  };

  const endDrag = () => {
    if (!drag) return;
    setDrag(null);
    if (drag.to === drag.from) return;
    const moved = moveRow(editRows, drag.from, drag.to);
    setEditRows(moved);
    const next = { ...positions };
    moved.forEach((row, index) => {
      if (row.entry && shifted(drag, index)) next[row.entry.entryId] = numberOf(row);
    });
    setPositions(next);
  };

  // The numbers are kept as typed: a gap (1, 2, 5) is volumes not catalogued yet (D61).
  const finishReorder = async () => {
    if (!shown) return;
    if (!shown.ordered) {
      setReorder(false);
      return;
    }
    const typed = shown.entries.map((e) => {
      const text = (positions[e.entryId] ?? "").trim();
      return { workId: e.workId, position: text === "" ? null : Number(text) };
    });
    // An ordered series shows only numbered works, so every one needs its number.
    if (typed.some((p) => p.position === null || !Number.isInteger(p.position) || p.position < 1)) {
      setPositionError(t("series.positionInvalid"));
      return;
    }
    setPositionError(null);
    await reorderSeries(shown.id, typed);
    setReorder(false);
    changed();
  };

  const removeWork = async (workId: string) => {
    if (!shown) return;
    await removeFromSeries(shown.id, workId);
    // Its row empties in the list being edited, which keeps what was dragged or typed for the others.
    setEditRows((current) =>
      current.map((row) => (row.entry?.workId === workId ? { ...row, entry: null } : row)),
    );
    changed();
  };

  const removeSeries = async () => {
    if (!shown) return;
    await deleteSeries(shown.id);
    setConfirmDelete(false);
    onClose();
    onChanged();
  };

  const openWork = (row: Row) => {
    if (!row.entry?.editionId) return;
    openOverlay({ name: "book", editionId: row.entry.editionId });
  };

  const openGap = (position: number) => {
    if (!shown) return;
    openOverlay({ name: "add", missing: true, intoSeries: { seriesId: shown.id, position } });
  };

  const rowView = (row: Row, index: number) => {
    const ref = (element: HTMLDivElement | null) => {
      rowElements.current[index] = element;
    };
    const dragging = drag !== null && drag.to === index;
    if (!row.entry) {
      // A gap: nothing catalogued for this number yet.
      const gapPosition = row.position;
      return (
        <div
          key={`gap-${row.position}-${index}`}
          ref={ref}
          className="flex items-center gap-3.5 rounded-xl border border-dashed bg-muted px-3 py-2"
        >
          {/* Lined up with the handles of the works around it. */}
          {draggable && <span className="w-6 shrink-0" />}
          {shown?.ordered && (
            <span
              className={cn(
                "flex h-[30px] shrink-0 items-center justify-center rounded-md border font-mono text-sm font-semibold",
                // As wide as the number fields of the works around it while editing.
                reorder ? "w-[52px]" : "w-[34px]",
              )}
            >
              {row.position}
            </span>
          )}
          <div className="flex min-w-0 flex-1 flex-col gap-0.5">
            <span className="text-sm font-semibold">{t("series.gap")}</span>
            <span className="text-[13px] text-muted-foreground">
              {t("series.gapHint", { n: row.position })}
            </span>
          </div>
          {!reorder && gapPosition !== null && (
            <Button variant="outline" size="sm" onClick={() => openGap(gapPosition)}>
              {t("series.addHere")}
            </Button>
          )}
        </div>
      );
    }
    const entry = row.entry;
    const owned = entry.copies > 0;
    const meta = [entry.publishedYear, entry.formats.map((f) => t(`formats.${f}`)).join(", ")]
      .filter(Boolean)
      .join(" · ");
    return (
      <div
        key={entry.entryId}
        ref={ref}
        className={cn(
          "flex items-center gap-3.5 rounded-xl border px-3 py-2",
          !owned && "border-dashed bg-muted",
          dragging && "relative z-10 bg-background shadow-lg ring-2 ring-primary",
        )}
      >
        {draggable && (
          <button
            type="button"
            aria-label={t("series.drag", { title: entry.title })}
            className="-mx-1 flex h-10 w-6 shrink-0 cursor-grab touch-none items-center justify-center rounded-md text-muted-foreground hover:bg-muted active:cursor-grabbing"
            onPointerDown={(event) => startDrag(event, index)}
            onPointerMove={moveDrag}
            onPointerUp={endDrag}
            onPointerCancel={() => setDrag(null)}
          >
            <GripVertical className="size-4" />
          </button>
        )}
        {shown?.ordered &&
          (reorder ? (
            <Input
              aria-label={t("series.position")}
              type="number"
              min={1}
              className="h-[30px] w-[52px] px-1.5 text-center font-mono"
              value={
                drag && shifted(drag, index) ? numberOf(row) : (positions[entry.entryId] ?? "")
              }
              onChange={(e) => setPositions({ ...positions, [entry.entryId]: e.target.value })}
            />
          ) : (
            <span className="flex h-[30px] w-[34px] shrink-0 items-center justify-center rounded-md border font-mono text-sm font-semibold">
              {entry.position}
            </span>
          ))}
        <button
          type="button"
          disabled={reorder}
          onClick={() => openWork(row)}
          className="flex min-w-0 flex-1 items-center gap-3.5 text-left"
        >
          <Cover file={entry.cover} alt={entry.title} kind={entry.kind} className="h-13" />
          <span className="flex min-w-0 flex-1 flex-col gap-0.5">
            <span className="truncate text-sm font-semibold">{entry.title}</span>
            {meta && <span className="truncate text-[12.5px] text-muted-foreground">{meta}</span>}
            {entry.needs && (
              <span className="flex items-center gap-1 text-[12.5px] text-amber-700 dark:text-amber-400">
                <TriangleAlert className="size-3.5 shrink-0" />
                <span className="truncate">{t("series.needsWork", { title: entry.needs })}</span>
              </span>
            )}
            {/* On a phone the status goes under the title, so the button beside it reads as what it is: an action. */}
            {!owned && (
              <span className="text-[12.5px] text-muted-foreground sm:hidden">
                {t("series.status.missing")}
              </span>
            )}
          </span>
        </button>
        <span
          className={cn(
            // Hidden below `sm`: next to the status button it would squeeze the title down
            // to a sliver on a phone (D61's mobile layout skips this badge altogether).
            "hidden w-24 shrink-0 text-right text-[12.5px] font-medium sm:inline",
            !owned && "text-muted-foreground",
          )}
        >
          {t(owned ? "series.status.owned" : "series.status.missing")}
        </span>
        {!reorder && !owned && (
          <Button
            variant="outline"
            size="sm"
            onClick={() =>
              row.entry?.editionId &&
              openOverlay({ name: "book", editionId: row.entry.editionId, addCopy: true })
            }
          >
            <Plus /> {t("series.iHaveIt")}
          </Button>
        )}
        {reorder && (
          <ConfirmButton
            trigger={
              <Button
                variant="ghost"
                size="icon-sm"
                className="shrink-0"
                aria-label={t("series.removeWork", { title: entry.title })}
              >
                <X />
              </Button>
            }
            title={t("series.removeConfirm.title", { title: entry.title })}
            description={t("series.removeConfirm.body", { name: shown?.name ?? "" })}
            confirmLabel={t("series.removeConfirm.action")}
            onConfirm={() => void removeWork(entry.workId)}
          />
        )}
      </div>
    );
  };

  const header = shown && (
    <div className="flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-end sm:justify-between">
      <div className="flex min-w-0 flex-col gap-1">
        <button
          type="button"
          className="self-start text-[13px] text-muted-foreground hover:underline"
          onClick={() => navigate("series")}
        >
          {t("nav.series")} ›
        </button>
        <h2 className="truncate text-2xl font-semibold tracking-tight">{shown.name}</h2>
        <span className="text-[13.5px] text-pretty text-muted-foreground">{summary}</span>
      </div>
      {reorder ? (
        <div className="flex items-center gap-2.5">
          <span className="hidden text-[13px] text-muted-foreground sm:inline">
            {t(shown.ordered ? "series.editHint" : "series.editHintUnordered")}
          </span>
          <Button size="sm" onClick={() => void finishReorder()}>
            <Check /> {t("series.done")}
          </Button>
        </div>
      ) : (
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" size="sm" onClick={() => setRenaming(true)}>
            <Pencil /> {t("series.rename")}
          </Button>
          {shown.entries.length > 0 && (
            <Button variant="outline" size="sm" onClick={startReorder}>
              <ListChecks /> {t("series.edit")}
            </Button>
          )}
          <Button size="sm" onClick={() => setAddingWork(true)}>
            <Plus /> {t("series.addWork")}
          </Button>
        </div>
      )}
    </div>
  );

  const body = (
    <div className="flex flex-col gap-4">
      {header}
      {reorder && positionError && <p className="text-[13px] text-destructive">{positionError}</p>}
      <div className="flex flex-col gap-1.5">{shownRows.map(rowView)}</div>
      {shown && !reorder && (
        <button
          type="button"
          className="flex items-center gap-2 self-start rounded-lg px-1 py-1.5 text-[13px] text-destructive hover:underline"
          onClick={() => setConfirmDelete(true)}
        >
          <Trash2 className="size-3.5" /> {t("series.delete")}
        </button>
      )}
      {shown && (
        <>
          <RenameSeriesDialog
            open={renaming}
            desktop={desktop}
            detail={shown}
            onClose={() => setRenaming(false)}
            onSaved={() => {
              setRenaming(false);
              changed();
            }}
          />
          <AddToSeriesWorkDialog
            open={addingWork}
            desktop={desktop}
            series={shown}
            onClose={() => setAddingWork(false)}
            onAdded={() => {
              setAddingWork(false);
              changed();
            }}
          />
          <ConfirmButton
            open={confirmDelete}
            onOpenChange={setConfirmDelete}
            title={t("series.deleteConfirm.title", { name: shown.name })}
            description={t("series.deleteConfirm.body")}
            confirmLabel={t("common.delete")}
            onConfirm={() => void removeSeries()}
          />
        </>
      )}
    </div>
  );

  if (desktop) {
    return (
      <Dialog open={seriesId !== null} onOpenChange={(open) => !open && onClose()}>
        <DialogContent
          showCloseButton={false}
          className="flex max-h-[85vh] w-[880px] max-w-[90vw] flex-col gap-0 overflow-hidden p-0 sm:max-w-[90vw]"
        >
          <DialogTitle className="sr-only">{shown?.name ?? t("nav.series")}</DialogTitle>
          <DialogDescription className="sr-only">{t("nav.series")}</DialogDescription>
          <div className="flex h-12 shrink-0 items-center justify-end px-3">
            <Button variant="ghost" size="icon-sm" aria-label={t("book.close")} onClick={onClose}>
              <X />
            </Button>
          </div>
          <div className="flex-1 overflow-y-auto px-6 pb-6">{body}</div>
        </DialogContent>
      </Dialog>
    );
  }

  return (
    <Dialog open={seriesId !== null} onOpenChange={(open) => !open && onClose()}>
      <ScreenDialogContent>
        <DialogTitle className="sr-only">{shown?.name ?? t("nav.series")}</DialogTitle>
        <DialogDescription className="sr-only">{t("nav.series")}</DialogDescription>
        <header className="flex h-16 shrink-0 items-center gap-1 px-1">
          <Button
            variant="ghost"
            size="icon-lg"
            className="size-12 rounded-full"
            aria-label={t("book.back")}
            onClick={onClose}
          >
            <ArrowLeft className="size-[22px]" />
          </Button>
        </header>
        <div className="flex-1 overflow-y-auto px-4 pb-[calc(2rem+env(safe-area-inset-bottom))]">
          {body}
        </div>
      </ScreenDialogContent>
    </Dialog>
  );
}

function RenameSeriesDialog({
  open,
  desktop,
  detail,
  onClose,
  onSaved,
}: {
  open: boolean;
  desktop: boolean;
  detail: SeriesDetail;
  onClose: () => void;
  onSaved: () => void;
}) {
  const { t } = useTranslation();
  const [name, setName] = useState(detail.name);
  const [saving, setSaving] = useState(false);
  const [shownOpen, setShownOpen] = useState(open);
  if (open !== shownOpen) {
    setShownOpen(open);
    if (open) setName(detail.name);
  }

  // Whether it is ordered is set once at creation; only the name changes here.
  const save = async () => {
    if (!name.trim()) return;
    setSaving(true);
    try {
      await updateSeries(detail.id, name.trim(), detail.ordered);
      onSaved();
    } finally {
      setSaving(false);
    }
  };

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={onClose}
      title={t("series.rename")}
      description={detail.name}
    >
      <div className="flex flex-col gap-4">
        <div className="space-y-2">
          <label htmlFor="series-rename-name" className="text-sm font-medium">
            {t("series.name")}
          </label>
          <Input
            id="series-rename-name"
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </div>
        <div className="flex justify-end gap-2">
          <Button type="button" variant="outline" onClick={onClose}>
            {t("form.cancel")}
          </Button>
          <Button type="button" disabled={saving || !name.trim()} onClick={() => void save()}>
            {saving ? t("form.saving") : t("form.save")}
          </Button>
        </div>
      </div>
    </SheetOrDialog>
  );
}
