import { useTranslation } from "react-i18next";

import { BookList } from "@/components/book-list";
import type { BookSummary } from "@/lib/api";

/**
 * "Missing" (D61): works catalogued with no copy yet, the catalog filtered by
 * `holding: "missing"`. A plain list for now; it is redesigned with series gaps
 * and "I wish it" (roadmap phase 6).
 */
export function MissingPage({
  books,
  desktop,
  onOpenBook,
}: {
  books: BookSummary[];
  desktop: boolean;
  onOpenBook: (book: BookSummary) => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex min-h-full flex-col gap-3">
      {desktop ? (
        <div className="flex items-center gap-2.5">
          <h1 className="text-[22px] font-semibold tracking-tight">{t("nav.missing")}</h1>
          <span className="text-[13px] text-muted-foreground">
            {t("nav.works", { count: books.length })}
          </span>
        </div>
      ) : (
        <p className="text-xs text-muted-foreground">{t("nav.works", { count: books.length })}</p>
      )}
      {books.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2.5 pt-10 pb-20 text-center">
          <p className="mt-1 text-[17px] font-semibold">{t("wishes.empty.title")}</p>
          <p className="max-w-[420px] text-sm text-pretty text-muted-foreground">
            {t("wishes.empty.body")}
          </p>
        </div>
      ) : (
        <BookList books={books} onOpen={onOpenBook} />
      )}
    </div>
  );
}
