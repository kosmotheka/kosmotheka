import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Circle, Drama, Gift, Heart, SlidersHorizontal, Tag as TagIcon, X } from "lucide-react";

import { useCatalogOptions } from "@/components/catalog-options";
import { STATUS_ICONS } from "@/components/marks/marks-section";
import { Switch } from "@/components/people/role-parts";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Sheet, SheetContent, SheetDescription, SheetTitle } from "@/components/ui/sheet";
import {
  type BookFilter,
  type Format,
  type Kind,
  type MarkStatus,
  type TagSummary,
  formats,
  genresOfKind,
  kinds,
  listTags,
  markStatuses,
} from "@/lib/api";
import { genreLabel, statusKey } from "@/lib/kinds";
import { closeOverlay, openOverlay } from "@/lib/navigation";
import { cn } from "@/lib/utils";

// Radix Select cannot use "" as a value, so "all" stands for "no filter".
const ALL = "all";

export function isFiltering(filter: BookFilter) {
  return Boolean(
    filter.query?.trim() ||
    filter.kind ||
    filter.format ||
    filter.owner ||
    filter.location ||
    filter.myStatuses?.length ||
    filter.myFavorites ||
    filter.myWished ||
    filter.othersWished ||
    filter.genres?.length ||
    filter.tags?.length,
  );
}

type MineOption = MarkStatus | "none";
const MINE_OPTIONS: MineOption[] = [...markStatuses, "none"];
const MINE_ICONS = { ...STATUS_ICONS, none: Circle };

/** One kind chip, plus "Todo"/"All" (`k === null`). Single choice: picking the active one clears it. */
const KIND_CHOICES: (Kind | null)[] = [null, ...kinds];

/** The kind chips, the "Filters" button and one removable chip per active filter. */
export function FilterBar({
  filter,
  onChange,
  desktop,
}: {
  filter: BookFilter;
  onChange: (filter: BookFilter) => void;
  desktop: boolean;
}) {
  const { t } = useTranslation();
  const [allTags, setAllTags] = useState<TagSummary[]>([]);
  useEffect(() => {
    listTags()
      .then(setAllTags)
      .catch(() => {});
  }, []);

  const chips: { key: string; label: string; remove: () => BookFilter }[] = [];
  const without = (key: keyof BookFilter) => () => ({ ...filter, [key]: undefined });
  if (filter.format) {
    chips.push({ key: "format", label: t(`formats.${filter.format}`), remove: without("format") });
  }
  if (filter.owner) chips.push({ key: "owner", label: filter.owner, remove: without("owner") });
  if (filter.location) {
    chips.push({ key: "location", label: filter.location, remove: without("location") });
  }
  for (const status of filter.myStatuses ?? []) {
    chips.push({
      key: `mine-${status}`,
      label: t("marks.filter.mine", {
        status:
          status === "none" ? t("marks.statuses.none") : t(statusKey(filter.kind ?? null, status)),
      }),
      remove: () => ({
        ...filter,
        myStatuses: filter.myStatuses?.filter((other) => other !== status),
      }),
    });
  }
  if (filter.myFavorites) {
    chips.push({
      key: "favorites",
      label: t("marks.filter.favorites"),
      remove: without("myFavorites"),
    });
  }
  for (const genre of filter.genres ?? []) {
    chips.push({
      key: `genre-${genre}`,
      label: filter.kind ? genreLabel(filter.kind, genre, t) : t(`genres.${genre}`),
      remove: () => ({ ...filter, genres: filter.genres?.filter((g) => g !== genre) }),
    });
  }
  for (const tagId of filter.tags ?? []) {
    chips.push({
      key: `tag-${tagId}`,
      label: allTags.find((tag) => tag.id === tagId)?.name ?? tagId,
      remove: () => ({ ...filter, tags: filter.tags?.filter((id) => id !== tagId) }),
    });
  }

  const filtersButton = (
    <Button
      variant="outline"
      className="h-9 shrink-0 rounded-[10px]"
      onClick={() => openOverlay({ name: "filters" })}
    >
      <SlidersHorizontal /> {t("filters.open")}
    </Button>
  );
  const kindChips = KIND_CHOICES.map((kind) => {
    const active = kind === null ? !filter.kind : filter.kind === kind;
    return (
      <Button
        key={kind ?? "all"}
        variant={active ? "default" : "outline"}
        className="h-9 shrink-0 rounded-[10px]"
        aria-pressed={active}
        onClick={() => onChange({ ...filter, kind: kind ?? undefined })}
      >
        {kind === null ? t("types.all") : t(`typesPlural.${kind}`)}
      </Button>
    );
  });

  // «Deseos» (D61): mine and others' wishes are independent, so both may be on at once.
  const wishOn = Boolean(filter.myWished || filter.othersWished);
  const wishLabel =
    filter.myWished && filter.othersWished
      ? t("filters.wish.chip")
      : filter.myWished
        ? t("filters.wish.mine")
        : filter.othersWished
          ? t("filters.wish.others")
          : t("filters.wish.chip");
  const wishChip = (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant={wishOn ? "default" : "outline"}
          className="h-9 shrink-0 rounded-[10px]"
          aria-pressed={wishOn}
        >
          <Gift /> {wishLabel}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        <DropdownMenuCheckboxItem
          checked={Boolean(filter.myWished)}
          onCheckedChange={(checked) => onChange({ ...filter, myWished: checked || undefined })}
        >
          {t("filters.wish.mine")}
        </DropdownMenuCheckboxItem>
        <DropdownMenuCheckboxItem
          checked={Boolean(filter.othersWished)}
          onCheckedChange={(checked) => onChange({ ...filter, othersWished: checked || undefined })}
        >
          {t("filters.wish.others")}
        </DropdownMenuCheckboxItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );

  return (
    <div className="flex items-center gap-2 overflow-x-auto">
      {!desktop && filtersButton}
      {kindChips}
      {wishChip}
      <GenreFilterChip filter={filter} onChange={onChange} />
      <TagFilterChip filter={filter} onChange={onChange} allTags={allTags} />
      {desktop && filtersButton}
      {chips.map((chip) => (
        <Button
          key={chip.key}
          className="h-9 shrink-0 rounded-[10px]"
          aria-label={t("filters.remove", { filter: chip.label })}
          onClick={() => onChange(chip.remove())}
        >
          {chip.label} <X />
        </Button>
      ))}
      {isFiltering({ ...filter, query: undefined }) && (
        <Button
          variant="link"
          className="h-9 shrink-0 px-1.5"
          onClick={() => onChange({ query: filter.query })}
        >
          {t("filters.clear")}
        </Button>
      )}
    </div>
  );
}

/** «Género» (6.7): genres of the active kind chip, or every kind's genres grouped when
 * no kind is active (Design's Catalog). */
function GenreFilterChip({
  filter,
  onChange,
}: {
  filter: BookFilter;
  onChange: (filter: BookFilter) => void;
}) {
  const { t } = useTranslation();
  const [groups, setGroups] = useState<{ kind: Kind; genres: string[] }[]>([]);

  useEffect(() => {
    let cancelled = false;
    const relevant = filter.kind ? [filter.kind] : kinds.filter((k) => k !== "other");
    Promise.all(
      relevant.map((kind) => genresOfKind(kind).then((genres) => ({ kind, genres }))),
    ).then((loaded) => {
      if (!cancelled) setGroups(loaded.filter((group) => group.genres.length > 0));
    });
    return () => {
      cancelled = true;
    };
  }, [filter.kind]);

  if (groups.length === 0) return null;

  const selected = filter.genres ?? [];
  const on = selected.length > 0;
  const toggle = (genre: string) =>
    onChange({
      ...filter,
      genres: selected.includes(genre) ? selected.filter((g) => g !== genre) : [...selected, genre],
    });
  const label =
    selected.length === 1
      ? filter.kind
        ? genreLabel(filter.kind, selected[0], t)
        : t(`genres.${selected[0]}`)
      : t("filters.genre.chip");

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant={on ? "default" : "outline"}
          className="h-9 shrink-0 rounded-[10px]"
          aria-pressed={on}
        >
          <Drama /> {label}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="max-h-80 w-64">
        {groups.map((group) => (
          <div key={group.kind}>
            {groups.length > 1 && (
              <div className="px-1.5 pt-1.5 pb-1 text-xs font-semibold text-muted-foreground">
                {t(`typesPlural.${group.kind}`)}
              </div>
            )}
            {group.genres.map((genre) => (
              <DropdownMenuCheckboxItem
                key={`${group.kind}-${genre}`}
                checked={selected.includes(genre)}
                onSelect={(e) => e.preventDefault()}
                onCheckedChange={() => toggle(genre)}
              >
                {genreLabel(group.kind, genre, t)}
              </DropdownMenuCheckboxItem>
            ))}
          </div>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** «Etiqueta» (6.7): every tag of this library, multi-select. */
function TagFilterChip({
  filter,
  onChange,
  allTags,
}: {
  filter: BookFilter;
  onChange: (filter: BookFilter) => void;
  allTags: TagSummary[];
}) {
  const { t } = useTranslation();
  if (allTags.length === 0) return null;

  const selected = filter.tags ?? [];
  const on = selected.length > 0;
  const toggle = (id: string) =>
    onChange({
      ...filter,
      tags: selected.includes(id) ? selected.filter((t) => t !== id) : [...selected, id],
    });
  const label =
    selected.length === 1
      ? (allTags.find((tag) => tag.id === selected[0])?.name ?? t("filters.tag.chip"))
      : t("filters.tag.chip");

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant={on ? "default" : "outline"}
          className="h-9 shrink-0 rounded-[10px]"
          aria-pressed={on}
        >
          <TagIcon /> {label}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="max-h-80 w-56">
        {allTags.map((tag) => (
          <DropdownMenuCheckboxItem
            key={tag.id}
            checked={selected.includes(tag.id)}
            onSelect={(e) => e.preventDefault()}
            onCheckedChange={() => toggle(tag.id)}
          >
            {tag.name}
          </DropdownMenuCheckboxItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
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

  const mine = filter.myStatuses ?? [];
  const toggleMine = (option: MineOption) =>
    set(
      "myStatuses",
      mine.includes(option) ? mine.filter((other) => other !== option) : [...mine, option],
    );

  const body = (
    <div className="flex flex-col gap-4 px-6">
      <div className="flex flex-col gap-2">
        <div className="flex items-baseline gap-2">
          <span className="text-sm font-medium">{t("marks.filter.title")}</span>
          <span className="text-xs text-muted-foreground">{t("marks.filter.hint")}</span>
        </div>
        <div className="flex flex-wrap gap-1.5">
          {MINE_OPTIONS.map((option) => {
            const on = mine.includes(option);
            const Icon = MINE_ICONS[option];
            return (
              <button
                key={option}
                type="button"
                role="checkbox"
                aria-checked={on}
                onClick={() => toggleMine(option)}
                className={cn(
                  "flex items-center gap-1.5 rounded-lg px-2.5 text-[13px]",
                  desktop ? "h-[30px]" : "h-10 rounded-[10px] px-3 text-sm",
                  on
                    ? "bg-primary/12 font-semibold shadow-[inset_0_0_0_1px_var(--primary)]"
                    : "shadow-[inset_0_0_0_1px_var(--border)]",
                )}
              >
                <Icon className={cn("size-3.5", !on && "text-muted-foreground")} />
                {option === "none"
                  ? t("marks.statuses.none")
                  : t(statusKey(filter.kind ?? null, option))}
              </button>
            );
          })}
        </div>
      </div>
      <label className="flex items-center gap-2.5">
        <Heart className="size-[15px]" fill="currentColor" />
        <span className="flex-1 text-sm font-medium">{t("marks.filter.favorites")}</span>
        <Switch
          checked={Boolean(filter.myFavorites)}
          label={t("marks.filter.favorites")}
          onChange={(on) => set("myFavorites", on || undefined)}
        />
      </label>
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
