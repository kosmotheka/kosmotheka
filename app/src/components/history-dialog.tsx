import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { type BookSummary, type HistoryEntry, bookHistory } from "@/lib/api";

interface Props {
  book: BookSummary | null;
  onClose: () => void;
}

export function HistoryDialog({ book, onClose }: Props) {
  const { t, i18n } = useTranslation();
  const [entries, setEntries] = useState<HistoryEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!book) return;
    let cancelled = false;
    bookHistory(book.editionId)
      .then((result) => !cancelled && setEntries(result))
      .catch((e: unknown) => !cancelled && setError(String(e)));
    return () => {
      cancelled = true;
      setEntries(null);
      setError(null);
    };
  }, [book]);

  const describe = (intent: string) =>
    t(`history.intents.${intent.replace(/\./g, "_")}`, { defaultValue: intent });

  const format = (timestamp: number) =>
    new Date(timestamp).toLocaleString(i18n.resolvedLanguage, {
      dateStyle: "medium",
      timeStyle: "short",
    });

  return (
    <Dialog open={book !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("history.title")}</DialogTitle>
          <DialogDescription>{book?.title}</DialogDescription>
        </DialogHeader>
        {error && (
          <p className="text-sm text-destructive">
            {t("errors.internal", { message: error })}
          </p>
        )}
        {entries?.length === 0 && (
          <p className="text-sm text-muted-foreground">{t("history.empty")}</p>
        )}
        {entries && entries.length > 0 && (
          <ol className="space-y-3 border-l pl-4">
            {entries.map((entry) => (
              <li key={entry.operationId}>
                <p className="text-sm font-medium">{describe(entry.intent)}</p>
                <p className="text-xs text-muted-foreground">
                  {format(entry.timestamp)}
                </p>
              </li>
            ))}
          </ol>
        )}
      </DialogContent>
    </Dialog>
  );
}
