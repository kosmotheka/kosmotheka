import { useTranslation } from "react-i18next";

import { Cover } from "@/components/cover";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { BookSummary } from "@/lib/api";

export function BookTable({
  books,
  onOpen,
}: {
  books: BookSummary[];
  onOpen: (book: BookSummary) => void;
}) {
  const { t } = useTranslation();

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead className="w-12">
            <span className="sr-only">{t("book.cover")}</span>
          </TableHead>
          <TableHead>{t("catalog.columns.title")}</TableHead>
          <TableHead>{t("catalog.columns.authors")}</TableHead>
          <TableHead className="hidden md:table-cell">
            {t("catalog.columns.year")}
          </TableHead>
          <TableHead>{t("catalog.columns.formats")}</TableHead>
          <TableHead className="hidden md:table-cell">
            {t("catalog.columns.owners")}
          </TableHead>
          <TableHead className="text-right">
            {t("catalog.columns.copies")}
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {books.map((book) => (
          <TableRow
            key={book.editionId}
            className="cursor-pointer"
            onClick={() => onOpen(book)}
          >
            <TableCell>
              <Cover file={book.cover} alt="" className="h-12 w-8" />
            </TableCell>
            <TableCell className="whitespace-normal">
              <button
                type="button"
                className="text-left font-medium hover:underline"
                onClick={(event) => {
                  event.stopPropagation();
                  onOpen(book);
                }}
              >
                {book.title}
              </button>
              {book.subtitle && (
                <div className="text-sm text-muted-foreground">
                  {book.subtitle}
                </div>
              )}
            </TableCell>
            <TableCell className="whitespace-normal">
              {book.authors.join(", ")}
            </TableCell>
            <TableCell className="hidden md:table-cell">
              {book.publishedYear}
            </TableCell>
            <TableCell>
              <div className="flex flex-wrap gap-1">
                {book.formats.map((format) => (
                  <Badge key={format} variant="secondary">
                    {t(`formats.${format}`)}
                  </Badge>
                ))}
              </div>
            </TableCell>
            <TableCell className="hidden whitespace-normal md:table-cell">
              {book.owners.join(", ")}
            </TableCell>
            <TableCell className="text-right">{book.copyCount}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
