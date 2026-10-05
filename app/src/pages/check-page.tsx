import { type SubmitEvent, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Bookmark,
  Check,
  LibraryBig,
  Loader2,
  ScanBarcode,
  Send,
  ShoppingBag,
  WifiOff,
  X,
} from "lucide-react";

import { Cover } from "@/components/cover";
import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { useRequests } from "@/components/requests-context";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  type EditionInput,
  type IsbnCheck,
  type LookupResult,
  addMissing,
  checkIsbn,
  isCommandError,
  isMobile,
  lookupIsbn,
  requestMissing,
} from "@/lib/api";
import { leaveSection, openOverlay } from "@/lib/navigation";
import { CameraDenied, readNotIsbn, scanIsbn, wantsToType } from "@/lib/scanner";

/** "Do I have it?": answered from the local catalog, so it works offline. */
export function CheckPage({
  desktop,
  version,
  onChanged,
  onOpenBook,
}: {
  desktop: boolean;
  /** Changes whenever the catalog does. */
  version: number;
  onChanged: () => void;
  /** Opens a book, switching to its library if it is in another one. */
  onOpenBook: (libraryId: string, editionId: string) => void;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const library = nameOf(libraries.active);
  const libraryName = (id: string) => nameOf(libraries.all.find((l) => l.id === id));
  const several = libraries.all.length > 1;
  const [isbn, setIsbn] = useState("");
  const [check, setCheck] = useState<IsbnCheck | null>(null);
  const [online, setOnline] = useState<LookupResult | null>(null);
  const [lookingUp, setLookingUp] = useState(false);
  const [offline, setOffline] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [canScan, setCanScan] = useState(false);
  const [wishTitle, setWishTitle] = useState("");
  const [wished, setWished] = useState(false);
  const [notIsbn, setNotIsbn] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  // Set by "I bought it" until the next scan, so the answer is checked
  // again once the catalog changes, instead of still saying "You don't have it".
  const recheck = useRef<string | null>(null);

  const reset = () => {
    recheck.current = null;
    setCheck(null);
    setOnline(null);
    setOffline(false);
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
      const missing = result.missing[0];
      if (missing) {
        setWished(true);
        setWishTitle(missing.title);
      }
      if (result.books.length === 0) {
        // Online data only improves the answer; without coverage it simply is not there.
        setLookingUp(true);
        lookupIsbn(result.isbn13)
          .then((found) => {
            if (found.found) {
              setOnline(found);
              if (!missing) setWishTitle(found.title ?? "");
            }
          })
          .catch(() => setOffline(true))
          .finally(() => setLookingUp(false));
      }
    } catch (e) {
      setError(
        isCommandError(e) && e.kind === "validation" ? t(`errors.${e.errors[0]?.code}`) : String(e),
      );
    }
  };

  const scan = async () => {
    try {
      setNotIsbn(false);
      const scanned = await scanIsbn();
      if (scanned) await run(scanned);
      else if (readNotIsbn()) {
        setNotIsbn(true);
        setTimeout(() => input.current?.focus(), 0);
      } else if (wantsToType()) setTimeout(() => input.current?.focus(), 0);
    } catch (e) {
      setError(e instanceof CameraDenied ? t("check.cameraDenied") : String(e));
    }
  };

  // On a phone the camera is the point of this screen, so it opens straight away.
  const started = useRef(false);
  useEffect(() => {
    void isMobile().then((mobile) => {
      setCanScan(mobile);
      if (mobile && !started.current) {
        started.current = true;
        void scan();
      }
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps -- only on arrival
  }, []);

  const again = () => {
    reset();
    setIsbn("");
    if (canScan) void scan();
    else setTimeout(() => input.current?.focus(), 0);
  };

  const submit = (event: SubmitEvent) => {
    event.preventDefault();
    if (isbn.trim()) void run(isbn.trim());
  };

  // Readers ask for it instead of adding it (D73).
  const reader = useRequests().myRole === "reader";

  /** Catalogs the work with no copy yet (D61): "I'm missing it". */
  const wish = async () => {
    if (!check || !wishTitle.trim()) return;
    const edition: EditionInput = {
      kind: "book",
      title: wishTitle.trim(),
      subtitle: null,
      authors: online?.authors ?? [],
      publisher: null,
      publishedYear: null,
      language: null,
      isbn: check.isbn13,
      coverUrl: online?.coverUrl ?? null,
      googleBooksId: null,
      openLibraryId: null,
    };
    if (reader) await requestMissing(edition, null);
    else await addMissing(edition);
    setWished(true);
    onChanged();
  };

  useEffect(() => {
    if (recheck.current) void checkIsbn(recheck.current).then(setCheck);
  }, [version]);

  const submitWish = (event: SubmitEvent) => {
    event.preventDefault();
    void wish();
  };

  const bought = () => {
    if (!check) return;
    recheck.current = check.isbn13;
    openOverlay({
      name: "add",
      prefill: {
        isbn: check.isbn13,
        title: online?.title ?? (wishTitle.trim() || undefined),
        author: online?.authors.join(", ") || undefined,
        found: online ?? undefined,
      },
    });
  };

  const owned = check && check.books.length > 0;
  const big = "h-[52px] rounded-[14px] text-[15px]";

  return (
    <div className="flex min-h-full flex-col gap-4">
      {desktop ? (
        <h1 className="text-[22px] font-semibold tracking-tight">{t("nav.check")}</h1>
      ) : (
        <header className="-mx-4 -mt-1 flex h-16 items-center gap-1 px-1">
          <Button
            variant="ghost"
            size="icon-lg"
            className="size-12 rounded-full"
            aria-label={t("common.close")}
            onClick={leaveSection}
          >
            <X className="size-[22px]" />
          </Button>
          <h1 className="text-lg font-semibold">{t("nav.check")}</h1>
        </header>
      )}

      <div className="flex w-full max-w-xl flex-col gap-4">
        {!check && (
          <>
            {notIsbn ? (
              <p className="rounded-[14px] border p-3 text-sm">{t("scanner.notIsbn")}</p>
            ) : (
              <p className="text-sm text-muted-foreground">{t("check.description")}</p>
            )}
            {canScan && (
              <Button className={big} onClick={() => void scan()}>
                <ScanBarcode /> {t("check.scan")}
              </Button>
            )}
            <form onSubmit={submit} className="flex gap-2">
              <Input
                ref={input}
                autoFocus={!canScan}
                inputMode="numeric"
                aria-label={t("form.fields.isbn")}
                placeholder={t("lookup.placeholder")}
                className="h-11"
                value={isbn}
                onChange={(e) => setIsbn(e.target.value)}
              />
              <Button type="submit" variant="secondary" className="h-11">
                {t("check.check")}
              </Button>
            </form>
          </>
        )}

        {error && <p className="text-sm text-destructive">{error}</p>}

        {owned && (
          <>
            <Verdict
              tone="yes"
              title={t("check.youHaveIt")}
              detail={
                several
                  ? t("check.inLibraries", {
                      count: new Set(check.books.map((b) => b.libraryId)).size,
                      total: libraries.all.length,
                    })
                  : t("check.inLibrary", { library })
              }
            />
            {check.books.map((book) => (
              <div key={book.editionId} className="flex flex-col gap-3">
                <button
                  type="button"
                  className="flex items-center gap-3.5 text-left"
                  onClick={() => onOpenBook(book.libraryId, book.editionId)}
                >
                  <Cover file={book.cover} alt="" className="h-16 w-11 rounded-md" />
                  <span className="flex min-w-0 flex-col gap-[3px]">
                    <span className="text-lg font-semibold">{book.title}</span>
                    <span className="text-[13px] text-muted-foreground">
                      {[book.authors.join(", "), book.publisher, book.publishedYear]
                        .filter(Boolean)
                        .join(" · ")}
                    </span>
                    <span className="font-mono text-xs text-muted-foreground">{check.isbn13}</span>
                  </span>
                </button>
                <div className="flex gap-3 rounded-2xl border p-3.5">
                  <LibraryBig className="size-5 shrink-0 text-muted-foreground" />
                  <div className="flex flex-col gap-0.5">
                    <span className="text-[15px] font-semibold">{libraryName(book.libraryId)}</span>
                    <span className="text-[13px]">
                      {book.formats.map((format) => t(`formats.${format}`)).join(" · ")}
                    </span>
                    <span className="text-[13px] text-muted-foreground">
                      {book.owners.length > 0 ? book.owners.join(", ") : t("copies.noOwner")}
                    </span>
                  </div>
                </div>
              </div>
            ))}
            <div className="grid grid-cols-2 gap-2.5">
              <Button
                variant="outline"
                className={big}
                onClick={() => {
                  const [book] = check.books;
                  if (book) onOpenBook(book.libraryId, book.editionId);
                }}
              >
                {t("check.openBook")}
              </Button>
              <Button className={big} onClick={again}>
                <ScanBarcode /> {t(canScan ? "check.scanAnother" : "check.checkAnother")}
              </Button>
            </div>
          </>
        )}

        {check && !owned && (
          <>
            <Verdict
              tone="no"
              title={t("check.notInCatalog")}
              detail={several ? t("check.inNoLibrary") : t("check.notInLibrary", { library })}
            />
            {lookingUp && (
              <p className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin" /> {t("check.lookingUp")}
              </p>
            )}
            {offline && (
              <p className="flex gap-2.5 text-[13px] leading-snug">
                <WifiOff className="mt-px size-[18px] shrink-0 text-muted-foreground" />
                <span>
                  <span className="font-medium">{t("check.offline")}</span>{" "}
                  <span className="text-muted-foreground">{t("check.offlineHint")}</span>
                </span>
              </p>
            )}
            <div className="flex items-center gap-3.5 rounded-[14px] border p-3">
              <Cover url={online?.coverUrl} alt="" className="h-[60px] w-10 rounded-md" />
              <div className="flex min-w-0 flex-1 flex-col gap-1.5">
                <span className="font-mono text-xs text-muted-foreground">ISBN {check.isbn13}</span>
                {online?.authors.length ? (
                  <span className="text-[13px] text-muted-foreground">
                    {online.authors.join(", ")}
                  </span>
                ) : null}
                <form id="wish" onSubmit={submitWish}>
                  <Input
                    aria-label={t("form.fields.title")}
                    placeholder={t("check.wishTitle")}
                    className="h-10"
                    enterKeyHint="done"
                    value={wishTitle}
                    onChange={(e) => setWishTitle(e.target.value)}
                  />
                </form>
              </div>
            </div>
            <div className="mt-1 flex flex-col gap-2.5">
              {wished ? (
                <p className="flex items-center justify-center gap-2 text-sm">
                  <Bookmark className="size-4" /> {t("check.alreadyWished")}
                </p>
              ) : (
                <Button type="submit" form="wish" className={big} disabled={!wishTitle.trim()}>
                  <Bookmark /> {t("check.addWish")}
                </Button>
              )}
              <div className="grid grid-cols-2 gap-2.5">
                <Button variant="outline" className={big} onClick={bought}>
                  {reader ? <Send /> : <ShoppingBag />}{" "}
                  {t(reader ? "ask.add" : "check.addToCatalog")}
                </Button>
                <Button variant="outline" className={big} onClick={again}>
                  <ScanBarcode /> {t(canScan ? "check.scanAnother" : "check.checkAnother")}
                </Button>
              </div>
              <span className="text-center text-xs text-muted-foreground">
                {t("check.savedIn", { library })}
              </span>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

function Verdict({ tone, title, detail }: { tone: "yes" | "no"; title: string; detail: string }) {
  const yes = tone === "yes";
  return (
    <div
      role="status"
      className={
        yes
          ? "flex items-center gap-3.5 rounded-[20px] bg-success-soft p-[18px] text-success"
          : "flex items-center gap-3.5 rounded-[20px] bg-muted p-[18px]"
      }
    >
      <span
        className={
          yes
            ? "flex size-12 shrink-0 items-center justify-center rounded-full bg-success text-background"
            : "flex size-12 shrink-0 items-center justify-center rounded-full border-2 border-foreground"
        }
      >
        {yes ? (
          <Check className="size-7" strokeWidth={3} />
        ) : (
          <X className="size-[26px]" strokeWidth={2.5} />
        )}
      </span>
      <span className="flex flex-col gap-0.5">
        <span className="text-2xl font-bold tracking-tight">{title}</span>
        <span className={yes ? "text-sm font-medium" : "text-sm text-muted-foreground"}>
          {detail}
        </span>
      </span>
    </div>
  );
}
