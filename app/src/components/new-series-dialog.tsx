import { type SubmitEvent, useState } from "react";
import { useTranslation } from "react-i18next";
import { ListOrdered, Layers } from "lucide-react";

import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { createSeries } from "@/lib/api";
import { closeOverlay, openOverlay } from "@/lib/navigation";
import { cn } from "@/lib/utils";

/** «Nueva serie» from the series list: name and ordered/unordered, nothing else to pick yet. */
export function NewSeriesDialog({ open, desktop }: { open: boolean; desktop: boolean }) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [ordered, setOrdered] = useState(true);
  const [saving, setSaving] = useState(false);

  const reset = () => {
    setName("");
    setOrdered(true);
    setSaving(false);
  };

  const submit = async (event: SubmitEvent) => {
    event.preventDefault();
    if (!name.trim()) return;
    setSaving(true);
    try {
      const id = await createSeries(name.trim(), ordered);
      reset();
      openOverlay({ name: "series", seriesId: id });
    } finally {
      setSaving(false);
    }
  };

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={() => {
        reset();
        closeOverlay();
      }}
      title={t("series.newSeriesTitle")}
      description={t("series.newSeriesTitle")}
    >
      <form onSubmit={(e) => void submit(e)} noValidate className="flex flex-col gap-4">
        <div className="space-y-2">
          <label htmlFor="new-series-name" className="text-sm font-medium">
            {t("series.name")}
          </label>
          <Input
            id="new-series-name"
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </div>
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
          <Button
            type="button"
            variant="outline"
            onClick={() => {
              reset();
              closeOverlay();
            }}
          >
            {t("form.cancel")}
          </Button>
          <Button type="submit" disabled={saving || !name.trim()}>
            {saving ? t("form.saving") : t("series.new")}
          </Button>
        </div>
      </form>
    </SheetOrDialog>
  );
}
