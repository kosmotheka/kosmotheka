import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { languageOf } from "@/i18n";
import {
  Camera,
  ChevronRight,
  Globe,
  Info,
  Loader2,
  PencilLine,
  ScanBarcode,
  Search,
  X,
} from "lucide-react";

import { AddBookDialog } from "@/components/add-book-dialog";
import { KindGrid } from "@/components/book-form";
import { Cover } from "@/components/cover";
import { type ProviderWarning, ProviderWarnings } from "@/components/provider-warnings";
import { ResultsCredits } from "@/components/source-credits";
import { useRequests } from "@/components/requests-context";
import { ShelfPhoto } from "@/components/shelf-photo";
import { ScreenDialogContent } from "@/components/shell/screen-dialog";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import {
  type Candidate,
  type CodeResult,
  type CreatedBook,
  type Kind,
  type NewBook,
  type Source,
  gameUpcChoice,
  isMobile,
  kinds,
  lookupCode,
  searchWorks,
} from "@/lib/api";
import { newBookFromCandidate } from "@/lib/candidates";
import { KIND_FORMATS, KIND_ICONS } from "@/lib/kinds";
import { IsbnChoice } from "@/components/isbn-choice";
import {
  CameraDenied,
  readIsbnChoices,
  readNoIsbnInPhoto,
  scanCode,
  wantsToType,
} from "@/lib/scanner";
import { openOverlay } from "@/lib/navigation";
import { cn } from "@/lib/utils";

/**
 * Providers each kind can be searched in without the cloud or an API key (D26).
 * A kind with fewer than two shows the "limited" notice; `book` always clears
 * that bar, so a Google Books key (set in Settings) never changes the outcome.
 */
const KIND_SOURCES: Record<Kind, Source[]> = {
  book: ["bne", "library_of_congress", "open_library", "inventaire", "wikidata"],
  comic: ["wikidata"],
  boardgame: ["wikidata"],
  film: ["wikidata"],
  tv_series: ["wikidata"],
  videogame: ["wikidata"],
  music: ["music_brainz"],
  other: [],
};

// Digits as printed on a product: EAN-8, ISBN-10, UPC-A (12) and EAN-13 or ISBN-13.
const CODE_LENGTHS = new Set([8, 10, 12, 13]);
// What a book's number looks like once its hyphens and spaces are taken out.
const ISBN_LENGTHS = new Set([10, 13]);
const DEBOUNCE_MS = 400;

/**
 * The code the search box holds, or null when it is a title. Only its shape is read here (digits, a
 * final X on ten, hyphens and spaces left out): whether it is a valid ISBN, and of what, is Rust's call.
 * Separated groups only count as a code with the length of an ISBN, so a title like "2010 2020" stays a title.
 */
function codeOf(text: string): string | null {
  const typed = text.trim();
  if (!/^[\d\s-]+X?$/i.test(typed)) return null;
  const code = typed.replace(/[\s-]/g, "").toUpperCase();
  const separated = code !== typed.toUpperCase();
  if (!(separated ? ISBN_LENGTHS : CODE_LENGTHS).has(code.length)) return null;
  return /^\d+$/.test(code) || /^\d{9}X$/.test(code) ? code : null;
}

function emptyNewBook(kind: Kind, barcode: string | null = null): NewBook {
  return {
    kind,
    title: "",
    subtitle: null,
    authors: [],
    contributors: [],
    publisher: null,
    publishedYear: null,
    language: null,
    isbn: null,
    barcode,
    number: null,
    playersMin: null,
    playersMax: null,
    durationMinutes: null,
    minAge: null,
    seasons: null,
    coverUrl: null,
    googleBooksId: null,
    openLibraryId: null,
    copy: {
      owner: null,
      format: KIND_FORMATS[kind][0],
      quantity: 1,
      location: null,
      acquiredOn: null,
      platform: null,
      seasons: null,
    },
  };
}

interface Props {
  open: boolean;
  desktop: boolean;
  library: string;
  onOpenChange: (open: boolean) => void;
  onCreated: (result: CreatedBook, title: string) => void;
  /** Works were added without leaving «Añadir» (the shelf photo): the catalogue behind refreshes. */
  onChanged?: () => void;
  /** A reader's book went to the administrators, or was simply added if allowed now. */
  onRequested?: (requested: boolean) => void;
  /** «Me falta» (D61): the form it leads to has no copy section, and saves with `addMissing`. */
  missing?: boolean;
}

/** «Añadir» starts here: search first, «Añadir a mano» always at hand (D60). */
export function AddSearch({
  open,
  desktop,
  library,
  onOpenChange,
  onCreated,
  onChanged,
  onRequested,
  missing,
}: Props) {
  const { t, i18n } = useTranslation();
  const [kind, setKind] = useState<Kind>("book");
  const [query, setQuery] = useState("");
  // Came from the scanner's «Type the number»: the field takes a code, with the number keyboard, and does not
  // search titles with each part of the number.
  const [typingCode, setTypingCode] = useState(false);
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const [warnings, setWarnings] = useState<ProviderWarning[]>([]);
  const [loading, setLoading] = useState(false);
  const [foundByCode, setFoundByCode] = useState(false);
  const [existingByCode, setExistingByCode] = useState<string | null>(null);
  // Set when a scanned/typed code has no clear kind: the "¿Qué es?" step.
  const [askCode, setAskCode] = useState<string | null>(null);
  // What Rust says of a typed number: written like an ISBN with a wrong check digit, or a valid ISBN (found or not).
  const [invalidIsbn, setInvalidIsbn] = useState(false);
  const [isbnByCode, setIsbnByCode] = useState<string | null>(null);
  const [formBook, setFormBook] = useState<NewBook | null>(null);
  // A barcode GameUPC was not sure of (D62), and how unsure: the board game the person picks for it, among its guesses or
  // searched by name, is sent back once it is saved, so GameUPC learns it.
  const [gameUpcCode, setGameUpcCode] = useState<string | null>(null);
  const [gameUpcStatus, setGameUpcStatus] = useState<"choose" | "unknown" | null>(null);
  const [pendingChoice, setPendingChoice] = useState<{ code: string; externalId: string } | null>(
    null,
  );
  const [scanning, setScanning] = useState(false);
  const [scanError, setScanError] = useState<string | null>(null);
  const [isbnChoices, setIsbnChoices] = useState<string[] | null>(null);
  const [shelfOpen, setShelfOpen] = useState(false);
  // The shelf photo adds straight away: not for «Me falta», nor for readers, who ask instead (D73).
  const { myRole } = useRequests();
  const canShelf = !missing && myRole !== "reader";
  // Where the spines can be read: a phone, whatever the window's width. Elsewhere the entry says to use one.
  const [phone, setPhone] = useState(false);
  useEffect(() => {
    void isMobile().then(setPhone);
  }, []);
  const requestId = useRef(0);
  const queryRef = useRef<HTMLInputElement>(null);

  const applyCodeResult = (trimmed: string, result: CodeResult) => {
    setFoundByCode(true);
    setExistingByCode(result.existingEditionId);
    setWarnings(result.warnings);
    setInvalidIsbn(result.invalidIsbn);
    setIsbnByCode(result.isbn);
    const unsure = result.gameUpc === "verified" ? null : result.gameUpc;
    setGameUpcCode(unsure ? trimmed : null);
    setGameUpcStatus(unsure);
    if (result.kind) {
      setKind(result.kind);
      setCandidates(result.candidates);
      setAskCode(null);
    } else if (result.invalidIsbn) {
      // Not an ISBN after all: say so, and do not ask what it is.
      setCandidates([]);
      setAskCode(null);
    } else {
      setCandidates([]);
      setAskCode(trimmed);
    }
  };

  const runSearch = async (forKind: Kind, text: string) => {
    const id = ++requestId.current;
    const trimmed = text.trim();
    setLoading(true);
    try {
      const code = codeOf(trimmed);
      if (code) {
        const result = await lookupCode(code);
        if (id === requestId.current) applyCodeResult(code, result);
      } else {
        setFoundByCode(false);
        setExistingByCode(null);
        setAskCode(null);
        setInvalidIsbn(false);
        setIsbnByCode(null);
        if (trimmed.length < 2 || (typingCode && /^[\d\s-]*$/.test(trimmed))) {
          setCandidates([]);
          setWarnings([]);
        } else {
          const result = await searchWorks(forKind, trimmed, languageOf(i18n.language));
          if (id === requestId.current) {
            setCandidates(result.candidates);
            setWarnings(result.warnings);
          }
        }
      }
    } finally {
      if (id === requestId.current) setLoading(false);
    }
  };

  // Debounced: re-runs on every kind or text change, including one set by scanning.
  useEffect(() => {
    if (!open) return;
    const timeout = setTimeout(() => void runSearch(kind, query), DEBOUNCE_MS);
    return () => clearTimeout(timeout);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [kind, query, open]);

  const handleScan = async () => {
    setScanError(null);
    if (!desktop) {
      setScanning(true);
      try {
        setIsbnChoices(null);
        const code = await scanCode();
        const choices = readIsbnChoices();
        if (code) setQuery(code);
        else if (choices) setIsbnChoices(choices);
        else if (readNoIsbnInPhoto()) {
          setScanError(t("scanner.noIsbnInPhoto"));
          queryRef.current?.focus();
        } else if (wantsToType()) {
          setTypingCode(true);
          setQuery("");
          setTimeout(() => queryRef.current?.focus(), 0);
        }
      } catch (error) {
        if (error instanceof CameraDenied) setScanError(t("addSearch.cameraDenied"));
      } finally {
        setScanning(false);
      }
      return;
    }
    // Desktop has no camera scanner: "scanning" is typing or pasting the code
    // into the same field, so the button just puts the cursor there.
    queryRef.current?.focus();
  };

  // A valid ISBN that nobody knows is surely a book: the form starts with its number.
  const handleManualAdd = () => {
    const book = emptyNewBook(kind, askCode ?? (kind === "boardgame" ? gameUpcCode : null));
    if (foundByCode && isbnByCode && kind === "book") book.isbn = isbnByCode;
    setFormBook(book);
  };

  const pickManualKindForCode = (picked: Kind) => {
    // GameUPC knew nothing of this code: a board game is searched by name, so that GameUPC can learn which it is.
    if (picked === "boardgame" && gameUpcStatus === "unknown") {
      setKind("boardgame");
      setAskCode(null);
      setQuery("");
      queryRef.current?.focus();
      return;
    }
    setFormBook(emptyNewBook(picked, askCode));
  };

  const dupOf = (candidate: Candidate): string | null =>
    candidate.existingEditionId ?? (foundByCode ? existingByCode : null);

  const pickCandidate = (candidate: Candidate) => {
    const existing = dupOf(candidate);
    if (existing) {
      openOverlay({ name: "book", editionId: existing });
      return;
    }
    const book = newBookFromCandidate(candidate);
    const { externalId } = candidate;
    const teaches =
      gameUpcCode !== null &&
      externalId !== null &&
      candidate.kind === "boardgame" &&
      (candidate.source === "game_upc" || candidate.source === "board_game_geek");
    if (teaches) book.barcode = gameUpcCode;
    setPendingChoice(teaches ? { code: gameUpcCode, externalId } : null);
    setFormBook(book);
  };

  // Saved (or asked for): only now does GameUPC hear which game it was. Best effort, and silent.
  const sendChoice = () => {
    if (pendingChoice)
      void gameUpcChoice(pendingChoice.code, pendingChoice.externalId).catch(() => {});
    setPendingChoice(null);
  };

  const sources = KIND_SOURCES[kind];
  const limited = sources.length === 1;

  if (formBook) {
    return (
      <AddBookDialog
        open={open}
        desktop={desktop}
        library={library}
        initial={formBook}
        missing={missing}
        onOpenChange={onOpenChange}
        onCreated={(result, title) => {
          sendChoice();
          onCreated(result, title);
        }}
        onRequested={(requested) => {
          sendChoice();
          onRequested?.(requested);
        }}
        onBack={() => {
          setPendingChoice(null);
          setFormBook(null);
        }}
      />
    );
  }

  const kindChips = (
    <div
      role="radiogroup"
      aria-label={t("form.fields.kind")}
      className={cn(
        "flex gap-1.5",
        desktop
          ? "flex-wrap"
          : // Sideways only for a finger; a narrow window with a mouse wraps, with no scrollbar over the chips.
            "overflow-x-auto [mask-image:linear-gradient(90deg,#000_85%,transparent)] pointer-fine:flex-wrap pointer-fine:overflow-x-visible pointer-fine:[mask-image:none]",
      )}
    >
      {kinds.map((k) => {
        const Icon = KIND_ICONS[k];
        // An ambiguous code has no kind yet: no chip is picked either (Design's "ask" state).
        const active = askCode === null && k === kind;
        return (
          <button
            key={k}
            type="button"
            role="radio"
            aria-checked={active}
            onClick={() => {
              setKind(k);
              // What GameUPC was unsure of is a board game: another type has nothing to teach it.
              if (k !== "boardgame") {
                setGameUpcCode(null);
                setGameUpcStatus(null);
              }
            }}
            className={cn(
              "flex h-9 shrink-0 items-center gap-1.5 rounded-lg px-3 text-[13px] font-medium",
              active
                ? "bg-primary text-primary-foreground"
                : "text-foreground ring-1 ring-border ring-inset",
            )}
          >
            <Icon className="size-3.5" />
            {t(`types.${k}`)}
          </button>
        );
      })}
    </div>
  );

  const searchField = (
    <div className="flex gap-2">
      <div
        role="search"
        className={cn(
          "flex h-11 flex-1 items-center gap-2.5 rounded-full bg-muted px-3.5",
          desktop && "rounded-lg ring-1 ring-ring",
        )}
      >
        {loading ? (
          <Loader2 className="size-[18px] shrink-0 animate-spin text-muted-foreground" />
        ) : codeOf(query) !== null ? (
          <ScanBarcode className="size-[18px] shrink-0 text-muted-foreground" />
        ) : (
          <Search className="size-[18px] shrink-0 text-muted-foreground" />
        )}
        <input
          ref={queryRef}
          aria-label={t(typingCode ? "addSearch.codePlaceholder" : "addSearch.placeholder")}
          placeholder={t(typingCode ? "addSearch.codePlaceholder" : "addSearch.placeholder")}
          inputMode={typingCode ? "numeric" : undefined}
          className="min-w-0 flex-1 bg-transparent text-[15px] outline-none placeholder:text-muted-foreground"
          value={query}
          onChange={(e) => {
            // Letters mean a title after all: back to searching as usual.
            if (/[a-wyz]/i.test(e.target.value)) setTypingCode(false);
            setQuery(e.target.value);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") void runSearch(kind, query);
          }}
        />
      </div>
      <Button
        type="button"
        variant={desktop ? "outline" : "default"}
        size={desktop ? "default" : "icon-lg"}
        className={desktop ? undefined : "size-11 shrink-0 rounded-full"}
        aria-label={t("nav.scan")}
        disabled={scanning}
        onClick={() => void handleScan()}
      >
        {scanning ? <Loader2 className="animate-spin" /> : <ScanBarcode />}
        {desktop && t("nav.scan")}
      </Button>
    </div>
  );

  const limitedNotice = limited && (
    <div className="flex gap-2 text-[13px] leading-snug text-pretty text-muted-foreground">
      <Info className="mt-0.5 size-4 shrink-0" />
      <span>
        {t("addSearch.limitedNotice", {
          kindPlural: t(`typesPlural.${kind}`).toLowerCase(),
          source: t(`lookup.sources.${sources[0]}`),
        })}
      </span>
    </div>
  );

  const askStep = askCode !== null && (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-1">
        <h2 className="text-lg font-semibold">{t("form.whatIsIt")}</h2>
        <p className="text-sm text-pretty text-muted-foreground">{t("addSearch.askHint")}</p>
      </div>
      <KindGrid value={null} onChange={pickManualKindForCode} columns={desktop ? 4 : 2} size="lg" />
    </div>
  );

  // What Rust made of the number typed: not an ISBN after all, or an ISBN no service has (add it by hand).
  const codeNote =
    askCode === null &&
    !loading &&
    foundByCode &&
    (invalidIsbn || (isbnByCode && candidates.length === 0)) ? (
      <p
        role="status"
        className={cn(
          "px-3 pt-1 text-sm",
          invalidIsbn ? "text-destructive" : "text-muted-foreground",
        )}
      >
        {t(invalidIsbn ? "addSearch.invalidIsbn" : "addSearch.isbnNotFound")}
      </p>
    ) : null;

  // GameUPC was not sure of the scanned code: say what picking a game does.
  const gameUpcNote = askCode === null && gameUpcCode !== null && kind === "boardgame" && (
    <div
      className={cn(
        "flex gap-2 pb-2 text-[13px] leading-snug text-pretty text-muted-foreground",
        desktop ? "px-3" : "px-2",
      )}
    >
      <Info className="mt-0.5 size-4 shrink-0" />
      <span>
        {t(
          gameUpcStatus === "choose" && foundByCode
            ? "addSearch.gameUpcChoose"
            : "addSearch.gameUpcSearch",
        )}
      </span>
    </div>
  );

  const resultsList = askCode === null && (
    <div className="flex flex-col gap-1">
      {desktop && candidates.length > 0 && (
        <span className="px-3 pb-1.5 text-xs font-medium text-muted-foreground">
          {foundByCode
            ? t("addSearch.foundByCode")
            : t("addSearch.resultCount", { count: candidates.length })}
        </span>
      )}
      {candidates.map((candidate, i) => {
        const existing = dupOf(candidate);
        const people =
          candidate.authors.length > 0
            ? candidate.authors
            : candidate.contributors.map((c) => c.name);
        const meta = [
          people.join(", "),
          candidate.publishedYear,
          candidate.publisher,
          candidate.durationMinutes
            ? `${candidate.durationMinutes} ${t("form.units.minutes")}`
            : null,
        ]
          .filter((part) => part !== null && part !== "")
          .join(" · ");
        return (
          <button
            key={`${candidate.source}-${candidate.externalId ?? i}`}
            type="button"
            onClick={() => pickCandidate(candidate)}
            className={cn(
              "flex items-center gap-3.5 rounded-xl p-2 text-left",
              desktop && i === 0 && "bg-muted",
            )}
          >
            <Cover
              url={candidate.coverUrl}
              alt={candidate.title}
              kind={candidate.kind}
              className={desktop ? "h-14" : "h-16"}
            />
            <div className="flex min-w-0 flex-1 flex-col gap-0.5">
              <span className="truncate text-[15px] font-semibold">{candidate.title}</span>
              {meta && <span className="truncate text-[13.5px] text-muted-foreground">{meta}</span>}
              {!desktop && (
                <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
                  <Globe className="size-3" />
                  {t(`lookup.sources.${candidate.source}`)}
                </span>
              )}
              {existing && (
                <span className="text-[13px] font-medium text-primary">
                  {t("addSearch.alreadyHere", { library })}
                </span>
              )}
            </div>
            {desktop && (
              <span className="shrink-0 rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground">
                {t(`lookup.sources.${candidate.source}`)}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );

  // Some sources ask for an active link to them on every page where their data is used (D62): the results are such a page.
  const sourceCredits = (
    <ResultsCredits sources={candidates.map((candidate) => candidate.source)} />
  );

  const warningsList = <ProviderWarnings warnings={warnings} />;

  // Right under the field: a failed photo of the number asks to type it there.
  const scanNote = isbnChoices ? (
    <IsbnChoice
      isbns={isbnChoices}
      onPick={(isbn) => {
        setIsbnChoices(null);
        setQuery(isbn);
      }}
    />
  ) : (
    scanError && <p className="rounded-[14px] border p-3 text-sm">{scanError}</p>
  );

  const footerHint = desktop && (
    <span className="flex-1 text-[13px] text-muted-foreground">
      {askCode !== null ? t("addSearch.askFooter") : t("addSearch.footerHint")}
    </span>
  );

  const manualButton = (
    <Button
      type="button"
      variant="outline"
      className={desktop ? "shrink-0" : "h-12 w-full rounded-full gap-2"}
      onClick={handleManualAdd}
    >
      <PencilLine className="size-4" /> {t("addSearch.manual")}
    </Button>
  );

  // Before anything is searched: the way to load a whole shelf at once.
  const shelfEntry = canShelf &&
    query.trim() === "" &&
    candidates.length === 0 &&
    askCode === null && (
      <>
        <button
          type="button"
          onClick={() => setShelfOpen(phone || !shelfOpen)}
          className="flex w-full items-center gap-3.5 rounded-xl p-3 text-left"
        >
          <span className="flex size-10 shrink-0 items-center justify-center rounded-full bg-muted">
            <Camera className="size-5" />
          </span>
          <span className="flex min-w-0 flex-1 flex-col gap-0.5">
            <span className="text-[15px] font-semibold">{t("shelf.title")}</span>
            <span className="text-[13.5px] text-pretty text-muted-foreground">
              {t("shelf.entry")}
            </span>
          </span>
          <ChevronRight className="size-4 shrink-0 text-muted-foreground" />
        </button>
        {!phone && shelfOpen && (
          <p role="status" className="mx-3 rounded-[14px] border p-3 text-sm text-pretty">
            {t("shelf.onlyOnPhone")}
          </p>
        )}
        <ShelfPhoto
          open={phone && shelfOpen}
          kind={kind}
          library={library}
          onOpenChange={setShelfOpen}
          onChanged={() => onChanged?.()}
          onSearch={(text) => {
            setTypingCode(false);
            setQuery(text);
          }}
        />
      </>
    );

  if (!desktop) {
    return (
      <Dialog open={open} onOpenChange={onOpenChange}>
        <ScreenDialogContent>
          <div className="flex min-h-0 flex-1 flex-col">
            <header className="flex h-16 shrink-0 items-center gap-1 px-1">
              <Button
                type="button"
                variant="ghost"
                size="icon-lg"
                className="size-12 rounded-full"
                aria-label={t("common.close")}
                onClick={() => onOpenChange(false)}
              >
                <X className="size-[22px]" />
              </Button>
              <DialogTitle className="flex-1 text-lg font-semibold">
                {t(missing ? "nav.addMissing" : "nav.add")}
              </DialogTitle>
            </header>
            <DialogDescription className="sr-only">{t("addSearch.placeholder")}</DialogDescription>
            <div className="flex flex-col gap-2.5 px-4">
              {searchField}
              {kindChips}
              {limitedNotice}
              {scanNote}
              {warningsList}
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto px-2 pt-3">
              {shelfEntry}
              {askStep}
              {gameUpcNote}
              {resultsList}
              {codeNote}
              {sourceCredits}
            </div>
            <div className="shrink-0 border-t px-4 pt-3 pb-[max(1.75rem,env(safe-area-inset-bottom))]">
              {manualButton}
            </div>
          </div>
        </ScreenDialogContent>
      </Dialog>
    );
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        showCloseButton
        className="flex max-h-[672px] w-full max-w-2xl flex-col gap-0 overflow-hidden p-0 sm:max-w-2xl"
      >
        <div className="flex items-center px-6 pt-4">
          <DialogTitle className="flex-1 text-lg font-semibold">
            {t(missing ? "form.missingTitle" : "form.title", { library })}
          </DialogTitle>
        </div>
        <DialogDescription className="sr-only">{t("addSearch.placeholder")}</DialogDescription>
        <div className="flex flex-col gap-3 px-6 pt-3.5">
          {kindChips}
          {searchField}
          {limitedNotice}
          {scanNote}
          {warningsList}
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-3 pt-3.5 pb-2">
          {shelfEntry}
          {askStep}
          {gameUpcNote}
          {resultsList}
          {codeNote}
          {sourceCredits}
        </div>
        <div className="mt-auto flex items-center gap-2.5 border-t px-6 py-3.5">
          {footerHint}
          {manualButton}
        </div>
      </DialogContent>
    </Dialog>
  );
}
