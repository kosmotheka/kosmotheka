import { useTranslation } from "react-i18next";
import { Gift } from "lucide-react";

import { Cover } from "@/components/cover";
import { MineIndicator } from "@/components/marks/mine-indicator";
import type { BookSummary, Kind } from "@/lib/api";
import { wishSummary } from "@/lib/kinds";

/** The catalog on mobile: one compact row per work, like an Android list. */
export function BookList({
  books,
  onOpen,
  activeKind = null,
  showWish = false,
}: {
  books: BookSummary[];
  onOpen: (book: BookSummary) => void;
  /** The kind chip selected, if any: drops the leading kind label from the meta line. */
  activeKind?: Kind | null;
  /** The «Me falta» view: a gift line with who wishes each work (D61). */
  showWish?: boolean;
}) {
  const { t } = useTranslation();
  return (
    <ul className="-mx-4">
      {books.map((book) => {
        const meta = [
          activeKind ? null : t(`types.${book.kind}`),
          book.authors.join(", "),
          book.publishedYear,
        ]
          .filter(Boolean)
          .join(" · ");
        return (
          <li key={book.editionId}>
            <button
              type="button"
              className="flex min-h-16 w-full items-center gap-3.5 px-4 py-2.5 text-left active:bg-muted"
              onClick={() => onOpen(book)}
            >
              <Cover file={book.cover} alt="" kind={book.kind} className="h-16 rounded-md" />
              <span className="flex min-w-0 flex-1 flex-col gap-[3px]">
                <span className="flex min-w-0 items-center gap-1.5">
                  <span className="truncate text-[15px] font-medium">{book.title}</span>
                  <MineIndicator book={book} activeKind={activeKind} large />
                </span>
                {meta && <span className="truncate text-[13px] text-muted-foreground">{meta}</span>}
                {showWish && book.wishedBy.length > 0 ? (
                  <span className="flex min-w-0 items-center gap-1.5 text-[13px] font-medium text-wish">
                    <Gift className="size-3.5 shrink-0" />
                    <span className="truncate">{wishSummary(t, book.wishedBy)}</span>
                  </span>
                ) : (
                  <span className="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
                    {book.formats.map((format) => (
                      <span
                        key={format}
                        className="shrink-0 rounded-md border px-[7px] py-px text-foreground"
                      >
                        {t(`formats.${format}`)}
                      </span>
                    ))}
                    <span className="truncate">{book.owners.join(", ")}</span>
                  </span>
                )}
              </span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}
