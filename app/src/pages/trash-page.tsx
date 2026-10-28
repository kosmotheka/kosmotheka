import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, BookCopy, BookOpen, ListOrdered, RotateCcw, Tag, Trash2 } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { ConfirmButton } from "@/components/confirm-button";
import { useRequests } from "@/components/requests-context";
import { SelectableRow, SelectionBar, useSelection } from "@/components/selection";
import { Button } from "@/components/ui/button";
import {
  type TrashItem,
  type TrashKind,
  type TrashRef,
  listTrash,
  purgeFromTrash,
  restoreFromTrash,
} from "@/lib/api";
import { leaveSection } from "@/lib/navigation";
import type { Notify } from "@/lib/notice";

const SECTIONS: { kind: TrashKind; icon: LucideIcon }[] = [
  { kind: "edition", icon: BookOpen },
  { kind: "copy", icon: BookCopy },
  { kind: "tag", icon: Tag },
  { kind: "series", icon: ListOrdered },
];

const DAY_MS = 24 * 3600_000;
const keyOf = (item: TrashRef) => `${item.kind}:${item.id}`;

/** Library › Trash (D105): what was deleted in the last 30 days, to restore it or delete it for good. */
export function TrashPage({
  desktop,
  version,
  onChanged,
  onNotice,
}: {
  desktop: boolean;
  version: number;
  onChanged: () => void;
  onNotice: Notify;
}) {
  const { t, i18n } = useTranslation();
  const { myRole } = useRequests();
  const admin = myRole === "admin";
  const [items, setItems] = useState<TrashItem[] | null>(null);
  // When the list was read: the days are counted from then, not on every render.
  const [readAt, setReadAt] = useState(0);
  const [purging, setPurging] = useState<TrashRef[] | null>(null);
  const selection = useSelection((items ?? []).map(keyOf));

  // Only the latest load counts: switching libraries can leave an older one in flight.
  const latest = useRef(0);
  const [reloads, setReloads] = useState(0);
  const reload = () => setReloads((n) => n + 1);
  useEffect(() => {
    const request = ++latest.current;
    listTrash()
      .then((found) => {
        if (request !== latest.current) return;
        setItems(found);
        setReadAt(Date.now());
      })
      .catch(() => {});
  }, [version, reloads]);

  const picked = (): TrashRef[] =>
    (items ?? [])
      .filter((item) => selection.selected.has(keyOf(item)))
      .map(({ kind, id }) => ({ kind, id }));

  const restore = async (refs: TrashRef[]) => {
    try {
      await restoreFromTrash(refs);
      selection.clear();
      onNotice(t("trash.restored", { count: refs.length }));
      reload();
      onChanged();
    } catch (e) {
      onNotice(String(e));
    }
  };

  const purge = async (refs: TrashRef[]) => {
    try {
      await purgeFromTrash(refs);
      selection.clear();
      onNotice(t("trash.purged", { count: refs.length }));
      reload();
      onChanged();
    } catch (e) {
      onNotice(String(e));
    }
  };

  const rtf = new Intl.RelativeTimeFormat(i18n.resolvedLanguage, { numeric: "auto" });
  const when = (at: string) => {
    const days = Math.round((Date.parse(at) - readAt) / DAY_MS);
    return days === 0 ? t("trash.today") : rtf.format(days, "day");
  };
  const daysLeft = (item: TrashItem) =>
    Math.max(0, Math.ceil((Date.parse(item.purgeAt) - readAt) / DAY_MS));

  const detail = (item: TrashItem) => {
    if (item.kind === "edition") {
      const kind = item.detail ? t(`types.${item.detail}`) : null;
      const copies = t("trash.copies", { count: item.count });
      return kind ? `${kind} · ${copies}` : copies;
    }
    if (item.kind === "copy") {
      const format = item.detail ? t(`formats.${item.detail}`) : null;
      return format ? t("trash.copyOf", { format }) : t("trash.copy");
    }
    return t("trash.works", { count: item.count });
  };

  const title = t("trash.title");

  return (
    <div className="flex min-h-full flex-col gap-6">
      {desktop ? (
        <div className="flex flex-col gap-0.5">
          <span className="text-[13px] text-muted-foreground">{t("trash.breadcrumb")}</span>
          <h1 className="text-[22px] font-semibold tracking-tight">{title}</h1>
        </div>
      ) : (
        <header className="-mx-4 -mt-1 flex h-16 items-center gap-1 px-1">
          <Button
            variant="ghost"
            size="icon-lg"
            className="size-12 rounded-full"
            aria-label={t("book.back")}
            onClick={leaveSection}
          >
            <ArrowLeft className="size-[22px]" />
          </Button>
          <h1 className="truncate text-lg font-semibold">{title}</h1>
        </header>
      )}

      <p className="max-w-2xl text-sm text-muted-foreground text-pretty">
        {admin ? t("trash.hintAdmin") : t("trash.hint")}
        {!desktop && items && items.length > 0 && ` ${t("trash.holdHint")}`}
      </p>

      <SelectionBar selection={selection}>
        <Button size="sm" variant="outline" onClick={() => void restore(picked())}>
          <RotateCcw className="size-[15px]" />
          {t("trash.restore")}
        </Button>
        {admin && (
          <Button size="sm" variant="destructive" onClick={() => setPurging(picked())}>
            <Trash2 className="size-[15px]" />
            {t("trash.purge")}
          </Button>
        )}
      </SelectionBar>

      {items?.length === 0 && (
        <div className="flex max-w-2xl flex-col items-center gap-2 py-10 text-center">
          <Trash2 className="size-8 text-muted-foreground" />
          <p className="text-sm font-medium">{t("trash.empty")}</p>
          <p className="text-sm text-muted-foreground text-pretty">{t("trash.emptyHint")}</p>
        </div>
      )}

      {SECTIONS.map(({ kind, icon: Icon }) => {
        const rows = (items ?? []).filter((item) => item.kind === kind);
        if (rows.length === 0) return null;
        return (
          <section key={kind} className="flex w-full max-w-2xl flex-col gap-2">
            <h2 className="text-[13px] font-semibold text-muted-foreground uppercase">
              {t(`trash.sections.${kind}`)} · {rows.length}
            </h2>
            <div className="flex flex-col border-t">
              {rows.map((item) => {
                const left = daysLeft(item);
                return (
                  <SelectableRow
                    key={keyOf(item)}
                    selection={selection}
                    id={keyOf(item)}
                    label={t("trash.select", { title: item.title })}
                    desktop={desktop}
                    actions={
                      <>
                        <Button
                          variant="ghost"
                          size={desktop ? "sm" : "icon"}
                          aria-label={t("trash.restoreAria", { title: item.title })}
                          onClick={(event) => {
                            event.stopPropagation();
                            void restore([{ kind: item.kind, id: item.id }]);
                          }}
                        >
                          <RotateCcw className="size-[15px]" />
                          {desktop && t("trash.restore")}
                        </Button>
                        {admin && (
                          <Button
                            variant="ghost"
                            size="icon"
                            aria-label={t("trash.purgeAria", { title: item.title })}
                            onClick={(event) => {
                              event.stopPropagation();
                              setPurging([{ kind: item.kind, id: item.id }]);
                            }}
                          >
                            <Trash2 className="size-[15px]" />
                          </Button>
                        )}
                      </>
                    }
                  >
                    <Icon className="size-[15px] shrink-0 text-muted-foreground" />
                    <div className="flex min-w-0 flex-1 flex-col py-2">
                      <span className="truncate text-sm font-medium">{item.title}</span>
                      <span className="truncate text-[13px] text-muted-foreground">
                        {detail(item)}
                      </span>
                      <span className="text-[13px] text-muted-foreground">
                        {item.trashedBy
                          ? t("trash.sentBy", { when: when(item.trashedAt), who: item.trashedBy })
                          : t("trash.sent", { when: when(item.trashedAt) })}
                        {" · "}
                        <span className={left <= 3 ? "text-destructive" : undefined}>
                          {left === 0 ? t("trash.goesSoon") : t("trash.goesIn", { count: left })}
                        </span>
                      </span>
                    </div>
                  </SelectableRow>
                );
              })}
            </div>
          </section>
        );
      })}

      <ConfirmButton
        open={purging !== null}
        onOpenChange={(open) => !open && setPurging(null)}
        title={t("trash.purgeTitle", { count: purging?.length ?? 0 })}
        description={t("trash.purgeBody", { count: purging?.length ?? 0 })}
        confirmLabel={t("trash.purge")}
        onConfirm={() => {
          if (purging) void purge(purging);
          setPurging(null);
        }}
      />
    </div>
  );
}
