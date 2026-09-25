import { type FormEvent, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { BookPlus, CircleCheck, CircleX, Heart, Loader2, ScanBarcode } from "lucide-react";

import { Cover } from "@/components/cover";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  type IsbnCheck,
  type LookupResult,
  addWish,
  checkIsbn,
  isCommandError,
  isMobile,
  lookupIsbn,
} from "@/lib/api";
import { CameraDenied, scanIsbn } from "@/lib/scanner";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onOpenBook: (editionId: string) => void;
  /** Opens "Add book" with what is known about this ISBN. */
  onAddToCatalog: (prefill: { isbn: string; title?: string; author?: string }) => void;
  onChanged: () => void;
}

/** "Do I already have it?": answered from the local catalog, so it works offline. */
export function CheckDialog({ open, onOpenChange, onOpenBook, onAddToCatalog, onChanged }: Props) {
  const { t } = useTranslation();
  const [isbn, setIsbn] = useState("");
  const [check, setCheck] = useState<IsbnCheck | null>(null);
  const [online, setOnline] = useState<LookupResult | null>(null);
  const [lookingUp, setLookingUp] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [canScan, setCanScan] = useState(false);
  const [wishTitle, setWishTitle] = useState("");
  const [wished, setWished] = useState(false);

  useEffect(() => {
    void isMobile().then(setCanScan);
  }, []);

  const reset = () => {
    setCheck(null);
    setOnline(null);
    setError(null);
    setWishTitle("");
    setWished(false);
  };

  const run = async (value: string) => {
    reset();
    setIsbn(value);
    try {
      const result = await checkIsbn(value);
      setCheck(result);
      if (result.books.length === 0) {
        // Online data only improves the answer; without coverage it simply is not there.
        setLookingUp(true);
        lookupIsbn(result.isbn13)
          .then((found) => {
            if (found.found) {
              setOnline(found);
              setWishTitle(found.title ?? "");
            }
          })
          .catch(() => {})
          .finally(() => setLookingUp(false));
      }
    } catch (e) {
      setError(
        isCommandError(e) && e.kind === "validation"
          ? t(`errors.${e.errors[0]?.code}`)
          : String(e),
      );
    }
  };

  const scan = async () => {
    try {
      const scanned = await scanIsbn();
      if (scanned) await run(scanned);
    } catch (e) {
      setError(e instanceof CameraDenied ? t("check.cameraDenied") : String(e));
    }
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (isbn.trim()) void run(isbn.trim());
  };

  const wish = async () => {
    if (!check || !wishTitle.trim()) return;
    await addWish({
      title: wishTitle.trim(),
      author: online?.authors.join(", ") || null,
      isbn: check.isbn13,
      coverUrl: online?.coverUrl ?? null,
    });
    setWished(true);
    onChanged();
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) {
          reset();
          setIsbn("");
        }
        onOpenChange(next);
      }}
    >
      <DialogContent className="max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{t("check.title")}</DialogTitle>
          <DialogDescription>{t("check.description")}</DialogDescription>
        </DialogHeader>

        {canScan && (
          <Button size="lg" onClick={() => void scan()}>
            <ScanBarcode /> {t("check.scan")}
          </Button>
        )}
        <form onSubmit={submit} className="flex gap-2">
          <Input
            autoFocus={!canScan}
            inputMode="numeric"
            aria-label={t("form.fields.isbn")}
            placeholder={t("lookup.placeholder")}
            value={isbn}
            onChange={(e) => setIsbn(e.target.value)}
          />
          <Button type="submit" variant="secondary">
            {t("check.check")}
          </Button>
        </form>

        {error && <p className="text-sm text-destructive">{error}</p>}

        {check && check.books.length > 0 && (
          <div className="space-y-3 rounded-lg border border-green-600/40 bg-green-600/10 p-4">
            <p className="flex items-center gap-2 font-medium">
              <CircleCheck className="size-5 text-green-600" /> {t("check.youHaveIt")}
            </p>
            {check.books.map((book) => (
              <button
                key={book.editionId}
                type="button"
                className="flex w-full gap-3 text-left"
                onClick={() => onOpenBook(book.editionId)}
              >
                <Cover file={book.cover} alt="" className="h-16 w-11" />
                <div className="space-y-1 text-sm">
                  <p className="font-medium">{book.title}</p>
                  <div className="flex flex-wrap gap-1">
                    {book.formats.map((format) => (
                      <Badge key={format} variant="secondary">
                        {t(`formats.${format}`)}
                      </Badge>
                    ))}
                  </div>
                  <p className="text-muted-foreground">
                    {book.owners.length > 0 ? book.owners.join(", ") : t("copies.noOwner")}
                  </p>
                </div>
              </button>
            ))}
          </div>
        )}

        {check && check.books.length === 0 && (
          <div className="space-y-3 rounded-lg border p-4">
            <p className="flex items-center gap-2 font-medium">
              <CircleX className="size-5 text-muted-foreground" /> {t("check.notInCatalog")}
            </p>
            {lookingUp && (
              <p className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin" /> {t("check.lookingUp")}
              </p>
            )}
            {online && (
              <div className="flex gap-3 text-sm">
                <Cover url={online.coverUrl} alt="" className="h-16 w-11" />
                <div>
                  <p className="font-medium">{online.title}</p>
                  <p className="text-muted-foreground">{online.authors.join(", ")}</p>
                </div>
              </div>
            )}
            {check.wishes.length > 0 || wished ? (
              <p className="flex items-center gap-2 text-sm">
                <Heart className="size-4 text-pink-600" /> {t("check.alreadyWished")}
              </p>
            ) : (
              <div className="flex gap-2">
                <Input
                  aria-label={t("form.fields.title")}
                  placeholder={t("check.wishTitle")}
                  value={wishTitle}
                  onChange={(e) => setWishTitle(e.target.value)}
                />
                <Button variant="outline" disabled={!wishTitle.trim()} onClick={() => void wish()}>
                  <Heart /> {t("check.addWish")}
                </Button>
              </div>
            )}
            <Button
              variant="secondary"
              onClick={() =>
                onAddToCatalog({
                  isbn: check.isbn13,
                  title: online?.title ?? (wishTitle.trim() || undefined),
                  author: online?.authors.join(", ") || undefined,
                })
              }
            >
              <BookPlus /> {t("check.addToCatalog")}
            </Button>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
