import { useCallback, useEffect, useState } from "react";
import i18n from "i18next";
import { useTranslation } from "react-i18next";
import {
  Calendar,
  ChevronDown,
  ChevronUp,
  CircleCheck,
  CircleDashed,
  CircleDot,
  CircleSlash,
  Heart,
  Info,
  Star,
} from "lucide-react";

import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { WhoDialog } from "@/components/marks/who-dialog";
import { Avatar } from "@/components/people/role-parts";
import {
  type Mark,
  type MarkStatus,
  type PersonMark,
  type WorkMarks,
  emptyMark,
  markStatuses,
  permissionErrorKey,
  setMark,
  workMarks,
} from "@/lib/api";
import { cn } from "@/lib/utils";

export const STATUS_ICONS = {
  pending: CircleDashed,
  in_progress: CircleDot,
  finished: CircleCheck,
  abandoned: CircleSlash,
} satisfies Record<MarkStatus, unknown>;

/**
 * «Mine» and «Everyone else's» in a work's page (Design, phase 5 §5.8, D68): each
 * person notes their status with its date, a rating and a favorite, whatever
 * their role, and sees what the others noted.
 */
export function MarksSection({
  workId,
  desktop,
  onNotice,
  onChanged,
}: {
  workId: string;
  desktop: boolean;
  onNotice: (message: string) => void;
  /** After saving, so the catalog shows the new status. */
  onChanged: () => void;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const library = useLibraryName()(libraries.active);
  const [marks, setMarks] = useState<WorkMarks | null>(null);
  // A change waiting for «Who are you?» to be answered.
  const [pending, setPending] = useState<Mark | null>(null);
  const [expanded, setExpanded] = useState(false);

  const load = useCallback(() => {
    workMarks(workId)
      .then(setMarks)
      .catch(() => setMarks(null));
  }, [workId]);
  useEffect(load, [load]);

  if (!marks) return null;
  const mine = marks.mine ?? emptyMark;

  const save = async (next: Mark) => {
    if (!marks.canMark) {
      if (marks.mayClaim) setPending(next);
      return;
    }
    setMarks({ ...marks, mine: next });
    try {
      await setMark(workId, next);
    } catch (e) {
      const key = permissionErrorKey(e);
      onNotice(key ? t(key) : String(e));
    }
    load();
    onChanged();
  };

  const chooseStatus = (status: MarkStatus) =>
    void save(
      // Tapping the current status again clears it; a new one is dated today.
      mine.status === status
        ? { ...mine, status: null, statusOn: null }
        : { ...mine, status, statusOn: today() },
    );
  const rate = (rating: number) =>
    void save({ ...mine, rating: mine.rating === rating ? null : rating });
  const marked = mine.status !== null;

  const block =
    !marks.canMark && !marks.mayClaim ? (
      <section
        aria-label={t("marks.mine")}
        aria-disabled
        className={cn(
          "flex flex-col gap-2.5 border border-dashed p-3",
          desktop ? "rounded-xl" : "rounded-2xl",
        )}
      >
        <div className="pointer-events-none flex flex-col gap-2.5 opacity-45">
          <span className={cn("font-semibold", desktop ? "text-[13px]" : "text-[15px]")}>
            {t("marks.mine")}
          </span>
          <StatusPicker status={null} desktop={desktop} onChoose={() => {}} />
        </div>
        <p className="flex items-start gap-2 text-[12.5px] leading-snug text-pretty">
          <Info className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
          {t(desktop ? "marks.lockedDevice" : "marks.lockedPhone", { library })}
        </p>
      </section>
    ) : (
      <section
        aria-label={t("marks.mine")}
        className={cn("flex flex-col", desktop ? "gap-2.5 rounded-xl border p-3" : "gap-2")}
      >
        <div className="flex min-h-7 items-center gap-2">
          <span className={cn("font-semibold", desktop ? "text-[13px]" : "text-[15px]")}>
            {t("marks.mine")}
          </span>
          <span className="flex-1" />
          {marked ? (
            <>
              <label
                className={cn(
                  "relative flex cursor-pointer items-center gap-1.5 rounded-md bg-muted px-2 font-medium",
                  desktop ? "h-[26px] text-xs" : "h-8 rounded-lg px-2.5 text-[13px]",
                )}
              >
                <Calendar className={desktop ? "size-[13px]" : "size-3.5"} />
                {dateLabel(t, mine)}
                <input
                  type="date"
                  aria-label={t("marks.changeDate")}
                  value={mine.statusOn ?? ""}
                  max={today()}
                  onChange={(e) =>
                    e.target.value && void save({ ...mine, statusOn: e.target.value })
                  }
                  className="absolute inset-0 cursor-pointer opacity-0"
                />
              </label>
              <button
                type="button"
                className={cn(
                  "px-1 font-medium text-primary",
                  desktop ? "text-xs" : "h-8 text-[13px]",
                )}
                onClick={() => void save(emptyMark)}
              >
                {t("marks.clear")}
              </button>
            </>
          ) : (
            <span className="text-xs whitespace-nowrap text-muted-foreground">
              {t("marks.hint")}
            </span>
          )}
        </div>
        <StatusPicker status={mine.status} desktop={desktop} onChoose={chooseStatus} />
        <div className="flex items-center gap-2.5">
          <div
            role="radiogroup"
            aria-label={t("marks.rating")}
            className={cn("flex", desktop ? "gap-1" : "-ml-2")}
          >
            {[1, 2, 3, 4, 5].map((n) => (
              <button
                key={n}
                type="button"
                role="radio"
                aria-checked={mine.rating === n}
                aria-label={t("marks.stars", { count: n })}
                onClick={() => rate(n)}
                className={cn(
                  "flex items-center justify-center",
                  !desktop && "h-11 w-10",
                  (mine.rating ?? 0) >= n ? "text-foreground" : "text-muted-foreground",
                )}
              >
                <Star
                  className={desktop ? "size-5" : "size-[26px]"}
                  fill={(mine.rating ?? 0) >= n ? "currentColor" : "none"}
                />
              </button>
            ))}
          </div>
          <span className="flex-1" />
          <button
            type="button"
            aria-pressed={mine.favorite}
            aria-label={t("marks.favorite")}
            onClick={() => void save({ ...mine, favorite: !mine.favorite })}
            className={cn(
              "flex items-center justify-center gap-1.5 border",
              desktop ? "h-8 rounded-lg px-2.5 text-[13px]" : "h-11 w-12 rounded-xl",
              mine.favorite ? "border-foreground font-semibold" : "border-border font-medium",
            )}
          >
            <Heart
              className={desktop ? "size-[15px]" : "size-[22px]"}
              fill={mine.favorite ? "currentColor" : "none"}
            />
            {desktop && t("marks.favorite")}
          </button>
        </div>
      </section>
    );

  return (
    <>
      {block}
      {marks.others.length > 0 && (
        <Others
          others={marks.others}
          desktop={desktop}
          expanded={expanded}
          onToggle={() => setExpanded(!expanded)}
        />
      )}
      {pending && (
        <WhoDialog
          desktop={desktop}
          onClose={() => setPending(null)}
          onClaimed={async () => {
            const next = pending;
            setPending(null);
            await setMark(workId, next).catch((e: unknown) => onNotice(String(e)));
            load();
            onChanged();
          }}
        />
      )}
    </>
  );
}

function StatusPicker({
  status,
  desktop,
  onChoose,
}: {
  status: MarkStatus | null;
  desktop: boolean;
  onChoose: (status: MarkStatus) => void;
}) {
  const { t } = useTranslation();
  return (
    <div
      role="radiogroup"
      aria-label={t("marks.status")}
      className={cn(
        "grid grid-cols-4 gap-0.5 bg-muted",
        desktop ? "rounded-[9px] p-0.5" : "rounded-[13px] p-[3px]",
      )}
    >
      {markStatuses.map((option) => {
        const on = option === status;
        const Icon = STATUS_ICONS[option];
        return (
          <button
            key={option}
            type="button"
            role="radio"
            aria-checked={on}
            title={on ? t("marks.tapAgain") : undefined}
            onClick={() => onChoose(option)}
            className={cn(
              "flex min-w-0 items-center justify-center px-0.5",
              desktop
                ? "h-[34px] gap-1 rounded-[7px] text-xs"
                : "h-11 flex-col gap-px rounded-[10px] text-[12.5px]",
              on
                ? "bg-background font-semibold text-foreground shadow-[0_1px_2px_oklch(0_0_0/14%),0_0_0_1px_var(--border)]"
                : "font-medium text-muted-foreground",
            )}
          >
            {on && <Icon className={desktop ? "size-3.5" : "size-[15px]"} />}
            <span className="truncate">{t(`marks.statuses.${option}`)}</span>
          </button>
        );
      })}
    </div>
  );
}

function Others({
  others,
  desktop,
  expanded,
  onToggle,
}: {
  others: PersonMark[];
  desktop: boolean;
  expanded: boolean;
  onToggle: () => void;
}) {
  const { t } = useTranslation();
  const first = others[0];
  if (!expanded) {
    return (
      <button
        type="button"
        aria-expanded={false}
        onClick={onToggle}
        className={cn(
          "-mt-2.5 flex items-center gap-2.5 text-left",
          desktop
            ? "rounded-lg px-2.5 py-2 text-[13px] hover:bg-muted"
            : "min-h-12 rounded-xl bg-muted px-3 text-[13.5px]",
        )}
      >
        <span className="flex">
          {others.slice(0, 3).map((o, i) => (
            <span
              key={o.person}
              className={cn(i > 0 && "-ml-1.5", "rounded-full ring-2 ring-background")}
            >
              <Avatar name={o.person} size={desktop ? 22 : 26} />
            </span>
          ))}
        </span>
        <span className="min-w-0 flex-1 leading-snug">
          {summary(t, first)}
          {others.length > 1 && (
            <span className="text-muted-foreground">
              {" "}
              {t("marks.andMore", { count: others.length - 1 })}
            </span>
          )}
        </span>
        <ChevronDown className="size-4 shrink-0 text-muted-foreground" />
      </button>
    );
  }
  return (
    <div
      className={cn(
        "-mt-2.5 flex flex-col bg-muted",
        desktop ? "gap-0.5 rounded-[10px] px-2.5 py-2" : "rounded-xl px-3 pt-1 pb-2",
      )}
    >
      <button
        type="button"
        aria-expanded
        onClick={onToggle}
        className="flex min-h-7 items-center gap-2 text-xs font-medium text-muted-foreground"
      >
        <span className="flex-1 text-left">{t("marks.others")}</span>
        <ChevronUp className="size-4" />
      </button>
      {others.map((o) => (
        <div
          key={o.person}
          className={cn("flex items-center gap-2.5", desktop ? "min-h-10" : "min-h-12")}
        >
          <Avatar name={o.person} size={desktop ? 22 : 26} />
          <span className="flex min-w-0 flex-1 flex-col gap-px">
            <span className={cn("font-medium", desktop ? "text-[13px]" : "text-sm")}>
              {o.person}
            </span>
            {o.status && <span className="text-xs text-muted-foreground">{detail(t, o)}</span>}
          </span>
          {o.rating ? (
            <span className="text-xs tracking-[1px]">{"★".repeat(o.rating)}</span>
          ) : (
            <span className="text-xs text-muted-foreground">{t("marks.unrated")}</span>
          )}
          {o.favorite && <Heart className="size-[13px]" fill="currentColor" />}
        </div>
      ))}
    </div>
  );
}

type T = ReturnType<typeof useTranslation>["t"];

/** Today as `YYYY-MM-DD`, in local time. */
function today() {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

function parse(date: string) {
  const [y, m, d] = date.split("-").map(Number);
  return new Date(y, m - 1, d);
}

/** «14 sep», with the year only when it is not this one. */
function shortDate(date: string, withYear = false) {
  const d = parse(date);
  const year = withYear || d.getFullYear() !== new Date().getFullYear();
  return d.toLocaleDateString(i18n.resolvedLanguage, {
    day: "numeric",
    month: "short",
    ...(year ? { year: "numeric" } : {}),
  });
}

/** «Since 20 Sep» while pending or in progress; «On 14 Sep» once it ended. */
function dateLabel(t: T, mark: Mark) {
  if (!mark.statusOn) return t("marks.noDate");
  const since = mark.status === "pending" || mark.status === "in_progress";
  return t(since ? "marks.since" : "marks.on", { date: shortDate(mark.statusOn) });
}

/** «Laura finished it in March · ★★★★★». */
function summary(t: T, o: PersonMark) {
  const month = o.statusOn
    ? parse(o.statusOn).toLocaleDateString(i18n.resolvedLanguage, { month: "long" })
    : "";
  const what = o.status
    ? t(`marks.summary.${o.status}${o.status === "finished" && month ? "In" : ""}`, {
        person: o.person,
        month,
      })
    : o.person;
  return o.rating ? `${what} · ${"★".repeat(o.rating)}` : what;
}

/** «Finished · 18 Mar 2026», «Reading · since 2 Sep». */
function detail(t: T, o: PersonMark) {
  const status = t(`marks.statuses.${o.status}`);
  if (!o.statusOn) return status;
  const since = o.status === "pending" || o.status === "in_progress";
  return since
    ? `${status} · ${t("marks.sinceLower", { date: shortDate(o.statusOn) })}`
    : `${status} · ${shortDate(o.statusOn, true)}`;
}
