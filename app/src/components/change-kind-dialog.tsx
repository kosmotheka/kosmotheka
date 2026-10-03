import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowRight, Check, TriangleAlert } from "lucide-react";

import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { BookDetail, Format, Kind } from "@/lib/api";
import type { ChangeKindItem, KindChangePlan } from "@/lib/change-kind";
import { KIND_FORMATS, copyWhereText } from "@/lib/kinds";

type Translate = ReturnType<typeof useTranslation>["t"];

function joinWithAnd(items: string[], t: Translate): string {
  if (items.length <= 1) return items[0] ?? "";
  return items.slice(0, -1).join(", ") + t("changeKind.and") + (items.at(-1) ?? "");
}

/** The label/value pair a kept or lost item reads as, e.g. "editorial (Devir)". */
function formatItem(item: ChangeKindItem, forKind: Kind, t: Translate): string {
  switch (item.field) {
    case "people":
      return `${t(`form.people.${item.role}`)} (${item.names.join(", ")})`;
    case "publisher":
      return `${t(forKind === "music" ? "form.fields.label" : "form.fields.publisher")} (${item.text})`;
    case "isbn":
      return `${t("form.fields.isbn")} (${item.text})`;
    case "barcode":
      return `${t("book.data.code")} (${item.text})`;
    case "language":
      return `${t("form.fields.language")} (${item.text})`;
    case "number":
      return `${t("form.fields.number")} (${item.text})`;
    case "players": {
      const value =
        item.min && item.max
          ? t("book.data.playersRange", { min: item.min, max: item.max })
          : String(item.min ?? item.max);
      return `${t("form.fields.players")} (${value})`;
    }
    case "duration":
      return `${t("form.fields.durationMinutes")} (${item.minutes} ${t("form.units.minutes")})`;
    case "minAge":
      return `${t("form.fields.minAge")} (${t("book.data.minAgeYears", { age: item.age })})`;
    case "seasons":
      return `${t("form.fields.seasons")} (${t("book.facts.seasons", { count: item.count })})`;
  }
}

/**
 * Confirms a kind change before it lands in the edit form: what moves, what is
 * lost, and what each copy's format becomes (Design's ChangeTypeSheet/Dialog).
 */
export function ChangeKindDialog({
  book,
  plan,
  desktop,
  onCancel,
  onConfirm,
}: {
  book: BookDetail;
  plan: KindChangePlan;
  desktop: boolean;
  onCancel: () => void;
  onConfirm: (copyFormats: Record<string, Format>) => void;
}) {
  const { t } = useTranslation();
  const [copyFormats, setCopyFormats] = useState<Record<string, Format>>(() =>
    Object.fromEntries(
      plan.copies.filter((c) => c.to !== null).map((c) => [c.copyId, c.to as Format]),
    ),
  );

  const fromLabel = t(`types.${plan.fromKind}`).toLowerCase();
  const toLabel = t(`types.${plan.toKind}`).toLowerCase();
  const heading =
    plan.toKind === "other"
      ? t("changeKind.titleOther")
      : t("changeKind.title", { from: fromLabel, to: toLabel });

  const keptText =
    plan.kept.length > 0
      ? joinWithAnd(
          plan.kept.map((i) => formatItem(i, plan.toKind, t)),
          t,
        )
      : null;
  const lostText =
    plan.lost.length > 0
      ? joinWithAnd(
          plan.lost.map((i) => formatItem(i, plan.fromKind, t)),
          t,
        )
      : null;

  const pending = plan.toKind !== "other" && plan.copies.some((c) => !copyFormats[c.copyId]);
  const loseAllData = plan.lost.length > 0 && (plan.toKind === "other" || plan.kept.length === 0);
  const cta = loseAllData ? t("changeKind.confirmLose") : t("changeKind.confirm", { to: toLabel });

  return (
    <SheetOrDialog
      open
      desktop={desktop}
      onClose={onCancel}
      title={heading}
      description={t("changeKind.description", { from: fromLabel })}
    >
      <p className="text-[13.5px] leading-snug text-pretty text-muted-foreground">
        {t("changeKind.description", { from: fromLabel })}
      </p>
      <div className="flex flex-col gap-2">
        {keptText && (
          <p className="flex items-start gap-2.5 text-sm leading-snug">
            <Check className="mt-0.5 size-[17px] shrink-0 text-success" strokeWidth={2.5} />
            <span>
              <span className="font-semibold">{t("changeKind.kept", { to: toLabel })}</span>
              {keptText}
            </span>
          </p>
        )}
        {lostText && (
          <p className="flex items-start gap-2.5 rounded-xl bg-warning-soft p-2.5 text-sm leading-snug">
            <TriangleAlert className="mt-0.5 size-[17px] shrink-0 text-warning" />
            <span>
              <span className="font-semibold">{t("changeKind.lost")}</span>
              {lostText}
            </span>
          </p>
        )}
      </div>
      <div className="flex flex-col gap-2">
        <span className="text-sm font-semibold">
          {t("changeKind.copiesHeading", { count: plan.copies.length })}
        </span>
        {plan.copies.map((copyPlan) => {
          const copy = book.copies.find((c) => c.id === copyPlan.copyId);
          if (!copy) return null;
          const selected = copyFormats[copyPlan.copyId] ?? null;
          return (
            <div key={copyPlan.copyId} className="flex items-center gap-2.5">
              <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                <span className="text-sm font-medium">{t(`formats.${copyPlan.from}`)}</span>
                <span className="truncate text-xs text-muted-foreground">
                  {copyWhereText(copy, t)}
                </span>
              </div>
              <ArrowRight className="size-4 shrink-0 text-muted-foreground" />
              {plan.toKind === "other" ? (
                <span className="w-32 shrink-0 text-sm text-muted-foreground">
                  {t("formats.other")}
                </span>
              ) : (
                <Select
                  value={selected ?? undefined}
                  onValueChange={(v) =>
                    setCopyFormats((prev) => ({ ...prev, [copyPlan.copyId]: v as Format }))
                  }
                >
                  <SelectTrigger
                    className={selected ? "w-32 shrink-0" : "w-32 shrink-0 ring-2 ring-ring"}
                  >
                    <SelectValue placeholder={t("changeKind.choose")} />
                  </SelectTrigger>
                  <SelectContent position="popper">
                    {KIND_FORMATS[plan.toKind].map((format) => (
                      <SelectItem key={format} value={format}>
                        {t(`formats.${format}`)}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            </div>
          );
        })}
        {pending && (
          <p className="text-xs text-muted-foreground">
            {t("changeKind.formatHint", {
              format: t(`formats.${plan.copies.find((c) => !copyFormats[c.copyId])?.from}`),
              kind: toLabel,
            })}
          </p>
        )}
      </div>
      <div className="flex flex-col gap-2 sm:flex-row-reverse">
        <Button disabled={pending} onClick={() => onConfirm(copyFormats)}>
          {cta}
        </Button>
        <Button variant="outline" onClick={onCancel}>
          {t("changeKind.keep", { from: fromLabel })}
        </Button>
      </div>
    </SheetOrDialog>
  );
}
