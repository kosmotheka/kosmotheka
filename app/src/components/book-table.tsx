import { useTranslation } from "react-i18next";
import { History } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
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
  onShowHistory,
}: {
  books: BookSummary[];
  onShowHistory: (book: BookSummary) => void;
}) {
  const { t } = useTranslation();

  return (
    <Table>
      <TableHeader>
        <TableRow>
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
          <TableHead className="w-10">
            <span className="sr-only">{t("history.title")}</span>
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {books.map((book) => (
          <TableRow key={book.editionId}>
            <TableCell className="whitespace-normal">
              <div className="font-medium">{book.title}</div>
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
            <TableCell>
              <Button
                variant="ghost"
                size="icon"
                aria-label={t("history.show", { title: book.title })}
                title={t("history.title")}
                onClick={() => onShowHistory(book)}
              >
                <History />
              </Button>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
