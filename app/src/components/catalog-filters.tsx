import { useTranslation } from "react-i18next";
import { Search, X } from "lucide-react";

import { useCatalogOptions } from "@/components/catalog-options";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { type BookFilter, type Format, formats } from "@/lib/api";

// Radix Select cannot use "" as a value, so "all" stands for "no filter".
const ALL = "all";

export function isFiltering(filter: BookFilter) {
  return Boolean(filter.query?.trim() || filter.format || filter.owner || filter.location);
}

export function CatalogFilters({
  filter,
  onChange,
}: {
  filter: BookFilter;
  onChange: (filter: BookFilter) => void;
}) {
  const { t } = useTranslation();
  const options = useCatalogOptions();
  const set = <K extends keyof BookFilter>(key: K, value: BookFilter[K]) =>
    onChange({ ...filter, [key]: value });

  return (
    <div className="flex flex-wrap items-center gap-2">
      <div className="relative min-w-56 flex-1">
        <Search className="absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          type="search"
          aria-label={t("filters.search")}
          placeholder={t("filters.searchPlaceholder")}
          className="pl-8"
          value={filter.query ?? ""}
          onChange={(e) => set("query", e.target.value)}
        />
      </div>
      <Select
        value={filter.format ?? ALL}
        onValueChange={(v) => set("format", v === ALL ? undefined : (v as Format))}
      >
        <SelectTrigger aria-label={t("form.fields.format")} className="w-auto min-w-40">
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
      {options.owners.length > 0 && (
        <Select
          value={filter.owner ?? ALL}
          onValueChange={(v) => set("owner", v === ALL ? undefined : v)}
        >
          <SelectTrigger aria-label={t("form.fields.owner")} className="w-auto min-w-40">
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
      )}
      {options.locations.length > 0 && (
        <Select
          value={filter.location ?? ALL}
          onValueChange={(v) => set("location", v === ALL ? undefined : v)}
        >
          <SelectTrigger aria-label={t("form.fields.location")} className="w-auto min-w-40">
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
      )}
      {isFiltering(filter) && (
        <Button variant="ghost" onClick={() => onChange({})}>
          <X /> {t("filters.clear")}
        </Button>
      )}
    </div>
  );
}
