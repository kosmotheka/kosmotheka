import { useTranslation } from "react-i18next";

import { Cover } from "@/components/cover";
import { Badge } from "@/components/ui/badge";
import type { BookSummary } from "@/lib/api";

/** The catalog on narrow screens, where a table does not fit. */
export function BookCards({
  books,
  onOpen,
}: {
  books: BookSummary[];
  onOpen: (book: BookSummary) => void;
}) {
  const { t } = useTranslation();
  return (
    <ul className="space-y-2">
      {books.map((book) => (
        <li key={book.editionId}>
          <button
            type="button"
            className="flex w-full gap-3 rounded-lg border p-3 text-left"
            onClick={() => onOpen(book)}
          >
            <Cover file={book.cover} alt="" className="h-20 w-14" />
            <div className="min-w-0 flex-1 space-y-1 text-sm">
              <p className="font-medium">{book.title}</p>
              {book.authors.length > 0 && (
                <p className="text-muted-foreground">{book.authors.join(", ")}</p>
              )}
              <div className="flex flex-wrap gap-1">
                {book.formats.map((format) => (
                  <Badge key={format} variant="secondary">
                    {t(`formats.${format}`)}
                  </Badge>
                ))}
              </div>
              {book.owners.length > 0 && (
                <p className="text-xs text-muted-foreground">{book.owners.join(", ")}</p>
              )}
            </div>
          </button>
        </li>
      ))}
    </ul>
  );
}
