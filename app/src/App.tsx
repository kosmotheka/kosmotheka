import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { BookPlus, CircleCheck } from "lucide-react";

import { AddBookDialog } from "@/components/add-book-dialog";
import { BookTable } from "@/components/book-table";
import { BookDialog } from "@/components/book-dialog";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { supportedLanguages } from "@/i18n";
import { type BookSummary, type CreatedBook, listBooks } from "@/lib/api";

type LoadState =
  | { status: "loading" }
  | { status: "ready"; books: BookSummary[] }
  | { status: "error"; message: string };

export default function App() {
  const { t, i18n } = useTranslation();
  const [state, setState] = useState<LoadState>({ status: "loading" });
  const [dialogOpen, setDialogOpen] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [openBookId, setOpenBookId] = useState<string | null>(null);

  const reload = useCallback(() => {
    listBooks()
      .then((books) => setState({ status: "ready", books }))
      .catch((error: unknown) =>
        setState({ status: "error", message: String(error) }),
      );
  }, []);

  useEffect(reload, [reload]);

  const books = useMemo(
    () => (state.status === "ready" ? state.books : []),
    [state],
  );
  const knownOwners = useMemo(
    () => [...new Set(books.flatMap((book) => book.owners))].sort(),
    [books],
  );

  const handleCreated = (result: CreatedBook, title: string) => {
    setNotice(
      t(result.existingEdition ? "catalog.addedCopy" : "catalog.added", {
        title,
      }),
    );
    reload();
  };

  const openDialog = () => {
    setNotice(null);
    setDialogOpen(true);
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6 p-4 md:p-8">
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold">{t("app.title")}</h1>
          {state.status === "ready" && (
            <p className="text-sm text-muted-foreground">
              {t("catalog.count", { count: books.length })}
            </p>
          )}
        </div>
        <div className="flex items-center gap-2">
          {supportedLanguages.map((language) => (
            <Button
              key={language}
              size="sm"
              variant={i18n.resolvedLanguage === language ? "secondary" : "ghost"}
              onClick={() => void i18n.changeLanguage(language)}
            >
              {language.toUpperCase()}
            </Button>
          ))}
          <Button onClick={openDialog}>
            <BookPlus /> {t("catalog.add")}
          </Button>
        </div>
      </header>

      {notice && (
        <Alert>
          <CircleCheck />
          <AlertDescription>{notice}</AlertDescription>
        </Alert>
      )}

      {state.status === "loading" && (
        <p className="text-muted-foreground">{t("catalog.loading")}</p>
      )}
      {state.status === "error" && (
        <p className="text-destructive">
          {t("errors.internal", { message: state.message })}
        </p>
      )}
      {state.status === "ready" &&
        (books.length === 0 ? (
          <div className="rounded-lg border border-dashed p-12 text-center">
            <p className="font-medium">{t("catalog.empty.title")}</p>
            <p className="mb-4 text-sm text-muted-foreground">
              {t("catalog.empty.body")}
            </p>
            <Button onClick={openDialog}>
              <BookPlus /> {t("catalog.add")}
            </Button>
          </div>
        ) : (
          <BookTable books={books} onOpen={(book) => setOpenBookId(book.editionId)} />
        ))}

      <AddBookDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        knownOwners={knownOwners}
        onCreated={handleCreated}
      />
      <BookDialog
        editionId={openBookId}
        knownOwners={knownOwners}
        onClose={() => setOpenBookId(null)}
        onChanged={reload}
      />
    </div>
  );
}
