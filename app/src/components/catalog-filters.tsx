import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { SlidersHorizontal, X } from "lucide-react";

import { useCatalogOptions } from "@/components/catalog-options";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Sheet, SheetContent, SheetDescription, SheetTitle } from "@/components/ui/sheet";
import { type BookFilter, type Format, formats } from "@/lib/api";
import { closeOverlay, openOverlay } from "@/lib/navigation";

// Radix Select cannot use "" as a value, so "all" stands for "no filter".
const ALL = "all";

export function isFiltering(filter: BookFilter) {
  return Boolean(filter.query?.trim() || filter.format || filter.owner || filter.location);
}

/** The "Filters" button and one removable chip per active filter. */
export function FilterBar({
  filter,
  onChange,
}: {
  filter: BookFilter;
  onChange: (filter: BookFilter) => void;
}) {
  const { t } = useTranslation();
  const chips: { key: keyof BookFilter; label: string }[] = [];
  if (filter.format) chips.push({ key: "format", label: t(`formats.${filter.format}`) });
  if (filter.owner) chips.push({ key: "owner", label: filter.owner });
  if (filter.location) chips.push({ key: "location", label: filter.location });

  return (
    <div className="flex items-center gap-2 overflow-x-auto">
      <Button
        variant="outline"
        className="h-9 shrink-0 rounded-[10px]"
        onClick={() => openOverlay({ name: "filters" })}
      >
        <SlidersHorizontal /> {t("filters.open")}
      </Button>
      {chips.map((chip) => (
        <Button
          key={chip.key}
          className="h-9 shrink-0 rounded-[10px]"
          aria-label={t("filters.remove", { filter: chip.label })}
          onClick={() => onChange({ ...filter, [chip.key]: undefined })}
        >
          {chip.label} <X />
        </Button>
      ))}
    </div>
  );
}

/** Format, owner and location: a bottom sheet on mobile, a dialog on desktop. */
export function FilterPanel({
  open,
  desktop,
  filter,
  onChange,
}: {
  open: boolean;
  desktop: boolean;
  filter: BookFilter;
  onChange: (filter: BookFilter) => void;
}) {
  const { t } = useTranslation();
  const options = useCatalogOptions();
  const set = <K extends keyof BookFilter>(key: K, value: BookFilter[K]) =>
    onChange({ ...filter, [key]: value });
  const onOpenChange = (next: boolean) => !next && closeOverlay();

  const body = (
    <div className="flex flex-col gap-4 px-6">
      <Field id="filter-format" label={t("form.fields.format")}>
        <Select
          value={filter.format ?? ALL}
          onValueChange={(v) => set("format", v === ALL ? undefined : (v as Format))}
        >
          <SelectTrigger id="filter-format" className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={ALL}>{t("filters.allFormats")}</SelectItem>
            {formats.map((format) => (
              <SelectItem key={format} value={format}>
                {t(`formats.${format}`)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      {options.owners.length > 0 && (
        <Field id="filter-owner" label={t("form.fields.owner")}>
          <Select
            value={filter.owner ?? ALL}
            onValueChange={(v) => set("owner", v === ALL ? undefined : v)}
          >
            <SelectTrigger id="filter-owner" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>{t("filters.allOwners")}</SelectItem>
              {options.owners.map((owner) => (
                <SelectItem key={owner} value={owner}>
                  {owner}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>
      )}
      {options.locations.length > 0 && (
        <Field id="filter-location" label={t("form.fields.location")}>
          <Select
            value={filter.location ?? ALL}
            onValueChange={(v) => set("location", v === ALL ? undefined : v)}
          >
            <SelectTrigger id="filter-location" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>{t("filters.allLocations")}</SelectItem>
              {options.locations.map((location) => (
                <SelectItem key={location} value={location}>
                  {location}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>
      )}
      <div className="mt-2 flex justify-end gap-2">
        {isFiltering({ ...filter, query: undefined }) && (
          <Button variant="outline" onClick={() => onChange({ query: filter.query })}>
            {t("filters.clear")}
          </Button>
        )}
        <Button onClick={closeOverlay}>{t("filters.done")}</Button>
      </div>
    </div>
  );

  if (desktop) {
    return (
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="gap-4 px-0 sm:max-w-sm">
          <DialogTitle className="px-6">{t("filters.open")}</DialogTitle>
          <DialogDescription className="sr-only">{t("filters.description")}</DialogDescription>
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
        className="gap-3 rounded-t-[28px] bg-popover pt-2.5 pb-[max(1.75rem,env(safe-area-inset-bottom))]"
      >
        <div className="mx-auto mb-1 h-1 w-8 rounded-full bg-muted-foreground/40" aria-hidden />
        <SheetTitle className="px-6 text-[17px] font-semibold">{t("filters.open")}</SheetTitle>
        <SheetDescription className="sr-only">{t("filters.description")}</SheetDescription>
        {body}
      </SheetContent>
    </Sheet>
  );
}

function Field({ id, label, children }: { id: string; label: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-2">
      <Label htmlFor={id}>{label}</Label>
      {children}
    </div>
  );
}
