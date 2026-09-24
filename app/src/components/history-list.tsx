import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { type HistoryEntry, bookHistory } from "@/lib/api";

/** `version` changes whenever the book changes, so the list reloads. */
export function HistoryList({
  editionId,
  version,
}: {
  editionId: string;
  version: number;
}) {
  const { t, i18n } = useTranslation();
  const [entries, setEntries] = useState<HistoryEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    bookHistory(editionId)
      .then((result) => !cancelled && setEntries(result))
      .catch((e: unknown) => !cancelled && setError(String(e)));
    return () => {
      cancelled = true;
    };
  }, [editionId, version]);

  const describe = (intent: string) =>
    t(`history.intents.${intent.replaceAll(".", "_")}`, {
      defaultValue: intent,
    });

  const format = (timestamp: number) =>
    new Date(timestamp).toLocaleString(i18n.resolvedLanguage, {
      dateStyle: "medium",
      timeStyle: "short",
    });

  if (error) {
    return (
      <p className="text-sm text-destructive">
        {t("errors.internal", { message: error })}
      </p>
    );
  }
  if (entries === null) return null;
  if (entries.length === 0) {
    return <p className="text-sm text-muted-foreground">{t("history.empty")}</p>;
  }
  return (
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
  );
}
