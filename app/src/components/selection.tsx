import { type PointerEvent, type ReactNode, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { useBackGuard } from "@/lib/navigation";

/** How long a finger stays down to start selecting, and how far it may move meanwhile. */
const HOLD_MS = 500;
const HOLD_SLOP_PX = 10;

/**
 * Picking several rows of a list to act on them at once (D105). On the computer each row has its
 * checkbox; on the phone a long press starts selecting, then a tap adds or removes, and back leaves the
 * selection before the screen. Keys that leave the list leave the selection.
 */
export function useSelection(keys: string[]) {
  const [picked, setSelected] = useState<ReadonlySet<string>>(new Set());
  const selected: ReadonlySet<string> = new Set(keys.filter((key) => picked.has(key)));
  const selecting = selected.size > 0;

  const clear = () => setSelected(new Set());
  useBackGuard(selecting, () => {
    clear();
    return false;
  });

  const toggle = (key: string) =>
    setSelected(() => {
      const next = new Set(selected);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  return {
    selected,
    selecting,
    toggle,
    clear,
    all: keys.length > 0 && keys.every((key) => selected.has(key)),
    selectAll: () => setSelected(new Set(keys)),
  };
}

export type Selection = ReturnType<typeof useSelection>;

/** Pointer handlers that call `onHold` after a finger or button stays down, and swallow the click that follows. */
function useLongPress(onHold: () => void) {
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const start = useRef<{ x: number; y: number } | null>(null);
  const held = useRef(false);

  const cancel = () => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
    start.current = null;
  };
  useEffect(() => cancel, []);

  return {
    onPointerDown: (event: PointerEvent) => {
      if (event.pointerType === "mouse" && event.button !== 0) return;
      held.current = false;
      start.current = { x: event.clientX, y: event.clientY };
      timer.current = setTimeout(() => {
        held.current = true;
        timer.current = null;
        onHold();
      }, HOLD_MS);
    },
    onPointerMove: (event: PointerEvent) => {
      if (!start.current) return;
      const moved = Math.hypot(event.clientX - start.current.x, event.clientY - start.current.y);
      if (moved > HOLD_SLOP_PX) cancel();
    },
    onPointerUp: cancel,
    onPointerCancel: cancel,
    onPointerLeave: cancel,
    // Android opens its own menu on a long press.
    onContextMenu: (event: { preventDefault: () => void }) => event.preventDefault(),
    /** True once, for the click that ends a long press. */
    consumeHold: () => {
      const was = held.current;
      held.current = false;
      return was;
    },
  };
}

/** One row that can be selected: its checkbox on the computer (or while selecting), long press on the phone. */
export function SelectableRow({
  selection,
  id,
  label,
  desktop,
  children,
  actions,
}: {
  selection: Selection;
  id: string;
  /** What the checkbox says to a screen reader. */
  label: string;
  desktop: boolean;
  children: ReactNode;
  /** Buttons at the end of the row, hidden while selecting on the phone. */
  actions?: ReactNode;
}) {
  const press = useLongPress(() => {
    if (!selection.selected.has(id)) selection.toggle(id);
  });
  const checked = selection.selected.has(id);
  const showBox = desktop || selection.selecting;
  return (
    <div
      data-selected={checked || undefined}
      className="flex min-h-15 items-center gap-3 border-b px-2 select-none data-selected:bg-muted/60"
      onPointerDown={desktop ? undefined : press.onPointerDown}
      onPointerMove={desktop ? undefined : press.onPointerMove}
      onPointerUp={desktop ? undefined : press.onPointerUp}
      onPointerCancel={desktop ? undefined : press.onPointerCancel}
      onPointerLeave={desktop ? undefined : press.onPointerLeave}
      onContextMenu={desktop ? undefined : press.onContextMenu}
      onClick={() => {
        if (press.consumeHold()) return;
        if (!desktop && selection.selecting) selection.toggle(id);
      }}
    >
      {showBox && (
        <input
          type="checkbox"
          aria-label={label}
          className="size-5 shrink-0 accent-primary"
          checked={checked}
          onClick={(event) => event.stopPropagation()}
          onChange={() => selection.toggle(id)}
        />
      )}
      {children}
      {(desktop || !selection.selecting) && actions}
    </div>
  );
}

/** The bar over a list while something is selected: how many, all or none, and what to do with them. */
export function SelectionBar({
  selection,
  children,
}: {
  selection: Selection;
  /** The actions on what is selected. */
  children: ReactNode;
}) {
  const { t } = useTranslation();
  if (!selection.selecting) return null;
  return (
    <div
      role="toolbar"
      aria-label={t("selection.toolbar")}
      className="sticky top-0 z-10 flex flex-wrap items-center gap-2 rounded-xl border bg-background px-2 py-1.5 shadow-sm"
    >
      <Button
        variant="ghost"
        size="icon"
        aria-label={t("selection.clear")}
        onClick={selection.clear}
      >
        <X className="size-[18px]" />
      </Button>
      <span className="text-sm font-medium tabular-nums">
        {t("selection.count", { count: selection.selected.size })}
      </span>
      <Button
        variant="ghost"
        size="sm"
        onClick={selection.all ? selection.clear : selection.selectAll}
      >
        {selection.all ? t("selection.none") : t("selection.all")}
      </Button>
      <span className="flex-1" />
      {children}
    </div>
  );
}
