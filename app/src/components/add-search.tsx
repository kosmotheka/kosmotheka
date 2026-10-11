import { useEffect, useRef, useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { Globe, Info, Loader2, PencilLine, ScanBarcode, Search, X } from "lucide-react";

import { AddBookDialog } from "@/components/add-book-dialog";
import { KindGrid } from "@/components/book-form";
import { Cover } from "@/components/cover";
import { ExternalLink, RAWG_URL } from "@/components/external-link";
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
  kinds,
  lookupCode,
  searchWorks,
} from "@/lib/api";
import { KIND_FORMATS, KIND_ICONS } from "@/lib/kinds";
import { CameraDenied, scanCode } from "@/lib/scanner";
import { openOverlay } from "@/lib/navigation";
import { cn } from "@/lib/utils";

/**
 * Providers each kind can be searched in without the cloud or an API key (D26).
 * A kind with fewer than two shows the "limited" notice; `book` always clears
 * that bar, so a Google Books key (set in Settings) never changes the outcome.
 */
const KIND_SOURCES: Record<Kind, Source[]> = {
  book: ["bne", "open_library", "inventaire", "wikidata"],
  comic: ["wikidata"],
  boardgame: ["wikidata"],
  film: ["wikidata"],
  tv_series: ["wikidata"],
  videogame: ["wikidata"],
  music: ["music_brainz"],
  other: [],
};

const CODE_LENGTHS = new Set([8, 12, 13]);
const DEBOUNCE_MS = 400;

function isCode(text: string): boolean {
  return /^\d+$/.test(text) && CODE_LENGTHS.has(text.length);
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

function newBookFromCandidate(candidate: Candidate): NewBook {
  const format =
    candidate.format && KIND_FORMATS[candidate.kind].includes(candidate.format)
      ? candidate.format
      : KIND_FORMATS[candidate.kind][0];
  return {
    kind: candidate.kind,
    title: candidate.title,
    subtitle: candidate.subtitle,
    authors: candidate.authors,
    contributors: candidate.contributors,
    publisher: candidate.publisher,
    publishedYear: candidate.publishedYear,
    language: candidate.language,
    isbn: candidate.isbn,
    barcode: candidate.barcode,
    number: candidate.number,
    playersMin: candidate.playersMin,
    playersMax: candidate.playersMax,
    durationMinutes: candidate.durationMinutes,
    minAge: candidate.minAge,
    seasons: candidate.seasons,
    coverUrl: candidate.coverUrl,
    googleBooksId: candidate.source === "google_books" ? candidate.externalId : null,
    openLibraryId: candidate.source === "open_library" ? candidate.externalId : null,
    copy: {
      owner: null,
      format,
      quantity: 1,
      location: null,
      acquiredOn: null,
      platform: candidate.platform,
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
  onRequested,
  missing,
}: Props) {
  const { t, i18n } = useTranslation();
  const [kind, setKind] = useState<Kind>("book");
  const [query, setQuery] = useState("");
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  type Warning = { source: Source; kind: "network" | "quota" | "bad_key" | "unexpected" };
  const [warnings, setWarnings] = useState<Warning[]>([]);
  const [loading, setLoading] = useState(false);
  const [foundByCode, setFoundByCode] = useState(false);
  const [existingByCode, setExistingByCode] = useState<string | null>(null);
  // Set when a scanned/typed code has no clear kind: the "¿Qué es?" step.
  const [askCode, setAskCode] = useState<string | null>(null);
  const [formBook, setFormBook] = useState<NewBook | null>(null);
  const [scanning, setScanning] = useState(false);
  const [scanError, setScanError] = useState<string | null>(null);
  const requestId = useRef(0);
  const queryRef = useRef<HTMLInputElement>(null);

  const applyCodeResult = (trimmed: string, result: CodeResult) => {
    setFoundByCode(true);
    setExistingByCode(result.existingEditionId);
    setWarnings(result.warnings);
    if (result.kind) {
      setKind(result.kind);
      setCandidates(result.candidates);
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
      if (isCode(trimmed)) {
        const result = await lookupCode(trimmed);
        if (id === requestId.current) applyCodeResult(trimmed, result);
      } else {
        setFoundByCode(false);
        setExistingByCode(null);
        setAskCode(null);
        if (trimmed.length < 2) {
          setCandidates([]);
          setWarnings([]);
        } else {
          const result = await searchWorks(
            forKind,
            trimmed,
            i18n.language.startsWith("es") ? "es" : "en",
          );
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
        const code = await scanCode();
        if (code) setQuery(code);
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

  const handleManualAdd = () => setFormBook(emptyNewBook(kind, askCode));

  const pickManualKindForCode = (picked: Kind) => setFormBook(emptyNewBook(picked, askCode));

  const dupOf = (candidate: Candidate): string | null =>
    candidate.existingEditionId ?? (foundByCode ? existingByCode : null);

  const pickCandidate = (candidate: Candidate) => {
    const existing = dupOf(candidate);
    if (existing) {
      openOverlay({ name: "book", editionId: existing });
      return;
    }
    setFormBook(newBookFromCandidate(candidate));
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
        onCreated={onCreated}
        onRequested={onRequested}
        onBack={() => setFormBook(null)}
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
          : "overflow-x-auto [mask-image:linear-gradient(90deg,#000_85%,transparent)]",
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
            onClick={() => setKind(k)}
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
        ) : isCode(query.trim()) ? (
          <ScanBarcode className="size-[18px] shrink-0 text-muted-foreground" />
        ) : (
          <Search className="size-[18px] shrink-0 text-muted-foreground" />
        )}
        <input
          ref={queryRef}
          aria-label={t("addSearch.placeholder")}
          placeholder={t("addSearch.placeholder")}
          className="min-w-0 flex-1 bg-transparent text-[15px] outline-none placeholder:text-muted-foreground"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
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

  // RAWG asks for an active link to it on every page where its data is used (D62): the results are such a page.
  const rawgAttribution = candidates.some((candidate) => candidate.source === "rawg") && (
    <p className="px-2 pt-1 text-xs text-muted-foreground">
      <Trans i18nKey="lookup.rawgData" components={{ link: <ExternalLink url={RAWG_URL} /> }} />
    </p>
  );

  const warningsList = warnings.length > 0 && (
    <div className="space-y-0.5 px-2 text-xs text-muted-foreground">
      {warnings.map((w, i) => (
        <p key={i}>
          {w.kind === "quota" && w.source === "google_books"
            ? t("lookup.googleQuota")
            : w.kind === "bad_key"
              ? t("lookup.badKey", { source: t(`lookup.sources.${w.source}`) })
              : t("lookup.unreachable", { source: t(`lookup.sources.${w.source}`) })}
        </p>
      ))}
      {scanError && <p className="text-destructive">{scanError}</p>}
    </div>
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
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto px-2 pt-3">
              {askStep}
              {resultsList}
              {rawgAttribution}
              {warningsList}
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
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-3 pt-3.5 pb-2">
          {askStep}
          {resultsList}
          {rawgAttribution}
          {warningsList}
        </div>
        <div className="mt-auto flex items-center gap-2.5 border-t px-6 py-3.5">
          {footerHint}
          {manualButton}
        </div>
      </DialogContent>
    </Dialog>
  );
}
