import { useTranslation } from "react-i18next";
import { Gift } from "lucide-react";

import { Cover } from "@/components/cover";
import { MineIndicator } from "@/components/marks/mine-indicator";
import type { BookSummary, Kind } from "@/lib/api";
import { wishSummary } from "@/lib/kinds";
import { cn } from "@/lib/utils";

/** The catalog on desktop. Rows are buttons, so the keyboard can open them too. */
export function BookTable({
  books,
  onOpen,
  selectedId,
  activeKind = null,
  showWish = false,
}: {
  books: BookSummary[];
  onOpen: (book: BookSummary) => void;
  /** The book open in the side panel: highlighted, and the table drops two columns to make room. */
  selectedId?: string | null;
  /** The kind chip selected, if any: shows this kind's status wording in `MineIndicator`. */
  activeKind?: Kind | null;
  /** The «Me falta» view: a column with who wishes each work (D61). */
  showWish?: boolean;
}) {
  const wide = !selectedId;
  const { t } = useTranslation();
  const row = "flex items-center gap-4 px-3";

  return (
    <div role="table" className="text-[13px]">
      <div role="row" className={`${row} h-9 border-b text-xs font-medium text-muted-foreground`}>
        <span className="w-10 shrink-0" role="columnheader">
          <span className="sr-only">{t("book.cover")}</span>
        </span>
        <span role="columnheader" className="min-w-0 flex-[2.4]">
          {t("catalog.columns.title")}
        </span>
        <span role="columnheader" className="w-24 shrink-0">
          {t("catalog.columns.type")}
        </span>
        <span role="columnheader" className="w-11 shrink-0">
          {t("catalog.columns.year")}
        </span>
        {wide && !showWish && (
          <>
            <span role="columnheader" className="w-36 shrink-0">
              {t("catalog.columns.formats")}
            </span>
            <span role="columnheader" className="min-w-0 flex-[1.4]">
              {t("catalog.columns.owners")}
            </span>
          </>
        )}
        {wide && showWish && (
          <span role="columnheader" className="min-w-0 flex-[1.4]">
            {t("filters.wish.chip")}
          </span>
        )}
      </div>
      {books.map((book) => (
        <button
          key={book.editionId}
          type="button"
          role="row"
          aria-current={book.editionId === selectedId || undefined}
          className={`${row} h-14 w-full border-b text-left hover:bg-accent focus-visible:bg-accent focus-visible:outline-none aria-[current]:bg-accent`}
          onClick={() => onOpen(book)}
        >
          <span className="flex w-10 shrink-0 justify-center">
            <Cover file={book.cover} alt="" kind={book.kind} className="h-11" />
          </span>
          <span className="flex min-w-0 flex-[2.4] flex-col gap-0.5">
            <span className="flex min-w-0 items-center gap-1.5">
              <span className="truncate text-sm font-medium">{book.title}</span>
              <MineIndicator book={book} activeKind={activeKind} />
            </span>
            <span className="truncate text-xs text-muted-foreground">
              {book.authors.join(", ")}
            </span>
          </span>
          <span className="w-24 shrink-0 truncate text-muted-foreground">
            {t(`types.${book.kind}`)}
          </span>
          <span className="w-11 shrink-0 text-muted-foreground tabular-nums">
            {book.publishedYear}
          </span>
          {wide && !showWish && (
            <>
              <span className="w-36 shrink-0 truncate">
                {book.formats.map((format) => t(`formats.${format}`)).join(", ")}
              </span>
              <span className="min-w-0 flex-[1.4] truncate text-muted-foreground">
                {book.owners.join(", ")}
                {book.copyCount > 1 && ` · ${t("catalog.copies", { count: book.copyCount })}`}
              </span>
            </>
          )}
          {wide && showWish && (
            <span
              className={cn(
                "flex min-w-0 flex-[1.4] items-center gap-1.5 truncate",
                book.wishedBy.includes("me") ? "font-medium text-wish" : "text-foreground",
              )}
            >
              {book.wishedBy.length > 0 ? (
                <>
                  <Gift className="size-3.5 shrink-0" />
                  {wishSummary(t, book.wishedBy)}
                </>
              ) : (
                <span className="text-muted-foreground">—</span>
              )}
            </span>
          )}
        </button>
      ))}
    </div>
  );
}
