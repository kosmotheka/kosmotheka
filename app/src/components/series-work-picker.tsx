import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Cover } from "@/components/cover";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { type BookSummary, type SeriesDetail, addToSeries, getBook, listBooks } from "@/lib/api";

/** «Añadir obra» on a series page: search the catalog for an existing work (not a new one),
 * then its number if the series is ordered. */
export function AddToSeriesWorkDialog({
  open,
  desktop,
  series,
  onClose,
  onAdded,
}: {
  open: boolean;
  desktop: boolean;
  series: SeriesDetail;
  onClose: () => void;
  onAdded: () => void;
}) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<BookSummary[]>([]);
  const [picked, setPicked] = useState<BookSummary | null>(null);
  const [position, setPosition] = useState("1");
  const [saving, setSaving] = useState(false);
  // Resets the form when the dialog opens again, the same render-phase trick as
  // `BookDetailView`'s `shownId` (no effect needed to mirror a prop into state).
  const [shownOpen, setShownOpen] = useState(open);

  // Matched by edition, not work id: a search result only carries the former.
  const inSeries = new Set(
    series.entries.map((e) => e.editionId).filter((id): id is string => id !== null),
  );

  if (open !== shownOpen) {
    setShownOpen(open);
    if (open) {
      setQuery("");
      setResults([]);
      setPicked(null);
      const max = Math.max(0, ...series.entries.map((e) => e.position ?? 0));
      setPosition(String(max + 1));
    }
  }

  useEffect(() => {
    if (!open || query.trim().length < 2) return;
    // An answer to an older query can arrive after the newer one: drop it.
    let current = true;
    const id = setTimeout(() => {
      listBooks({ query: query.trim() })
        .then((found) => current && setResults(found))
        .catch(() => current && setResults([]));
    }, 300);
    return () => {
      current = false;
      clearTimeout(id);
    };
  }, [open, query]);

  const add = async () => {
    if (!picked) return;
    setSaving(true);
    try {
      // A search result only has its edition: the work it belongs to is looked up first.
      const { workId } = await getBook(picked.editionId);
      const n = Number(position);
      await addToSeries(
        series.id,
        workId,
        series.ordered && Number.isFinite(n) && n > 0 ? n : null,
      );
      onAdded();
    } finally {
      setSaving(false);
    }
  };

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={onClose}
      title={t("series.workPicker.title")}
      description={t("series.workPicker.title")}
    >
      {!picked ? (
        <div className="flex flex-col gap-3">
          <Input
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("series.workPicker.placeholder")}
          />
          <div className="flex flex-col gap-1">
            {(query.trim().length < 2 ? [] : results).map((book) => {
              const already = inSeries.has(book.editionId);
              return (
                <button
                  key={book.editionId}
                  type="button"
                  disabled={already}
                  onClick={() => setPicked(book)}
                  className="flex items-center gap-3 rounded-lg p-1.5 text-left disabled:opacity-50"
                >
                  <Cover file={book.cover} alt={book.title} kind={book.kind} className="h-12" />
                  <div className="flex min-w-0 flex-1 flex-col">
                    <span className="truncate text-sm font-semibold">{book.title}</span>
                    {already && (
                      <span className="text-xs text-muted-foreground">
                        {t("series.workPicker.alreadyIn")}
                      </span>
                    )}
                  </div>
                </button>
              );
            })}
          </div>
          <div className="flex justify-end">
            <Button type="button" variant="outline" onClick={onClose}>
              {t("form.cancel")}
            </Button>
          </div>
        </div>
      ) : (
        <div className="flex flex-col gap-4">
          <div className="flex items-center gap-3 rounded-lg border p-2">
            <Cover file={picked.cover} alt={picked.title} kind={picked.kind} className="h-12" />
            <span className="min-w-0 flex-1 truncate text-sm font-semibold">{picked.title}</span>
          </div>
          {series.ordered && (
            <div className="space-y-2">
              <label htmlFor="series-work-position" className="text-sm font-medium">
                {t("series.position")}
              </label>
              <Input
                id="series-work-position"
                type="number"
                min={1}
                className="w-20 font-mono"
                value={position}
                onChange={(e) => setPosition(e.target.value)}
              />
            </div>
          )}
          <div className="flex justify-end gap-2">
            <Button type="button" variant="outline" onClick={() => setPicked(null)}>
              {t("form.cancel")}
            </Button>
            <Button type="button" disabled={saving} onClick={() => void add()}>
              {saving ? t("form.saving") : t("series.addWork")}
            </Button>
          </div>
        </div>
      )}
    </SheetOrDialog>
  );
}
