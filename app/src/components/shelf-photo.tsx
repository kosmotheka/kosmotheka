// The shelf photo, a help for the first load: the spines are read on the phone, each text is searched in the
// catalogues, and only what matches a real work is offered, to tick and add at once with the same copy details.

import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Camera, CircleAlert, CircleCheck, ImageIcon, Loader2, Search, X } from "lucide-react";

import { ChoiceSelect } from "@/components/choice-select";
import { Field } from "@/components/book-form";
import { useCatalogOptions } from "@/components/catalog-options";
import { SuggestInput } from "@/components/suggest-input";
import { Cover } from "@/components/cover";
import { ScreenDialogContent } from "@/components/shell/screen-dialog";
import { ResultsCredits } from "@/components/source-credits";
import { Button } from "@/components/ui/button";
import { Dialog, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { languageOf } from "@/i18n";
import {
  type Candidate,
  type Format,
  type Kind,
  createBook,
  restoreFromTrash,
  kinds,
  shelfMatch,
  shelfQueries,
} from "@/lib/api";
import { newBookFromCandidate } from "@/lib/candidates";
import { KIND_FORMATS } from "@/lib/kinds";
import { takePhoto } from "@/lib/photo";

// A few searches at a time: quicker than one by one, and gentle with the catalogues and the server's limit.
const AT_ONCE = 3;

type Step = "guide" | "reading" | "searching" | "review";

interface Found {
  candidate: Candidate;
  read: string;
  picked: boolean;
  status?: "adding" | "added" | "failed";
}

interface Props {
  open: boolean;
  /** What «Añadir» was set to: a shelf is usually all of one kind. */
  kind: Kind;
  library: string;
  onOpenChange: (open: boolean) => void;
  /** Called after each work added, so the catalogue refreshes behind. */
  onChanged: () => void;
  /** A text read but not found, to search by hand in «Añadir». */
  onSearch: (text: string) => void;
}

export function ShelfPhoto({
  open,
  kind: initialKind,
  library,
  onOpenChange,
  onChanged,
  onSearch,
}: Props) {
  const { t, i18n } = useTranslation();
  const [step, setStep] = useState<Step>("guide");
  const [kind, setKind] = useState<Kind>(initialKind);
  const [failed, setFailed] = useState(false);
  const [queries, setQueries] = useState<string[]>([]);
  const [searched, setSearched] = useState(0);
  const [found, setFound] = useState<Found[]>([]);
  const [notFound, setNotFound] = useState<string[]>([]);
  const [owner, setOwner] = useState("");
  const [format, setFormat] = useState<Format>(KIND_FORMATS[initialKind][0]);
  const [location, setLocation] = useState("");
  const [adding, setAdding] = useState(false);
  const options = useCatalogOptions();
  // Bumped on close or a new photo, so searches still running for the old one are dropped.
  const run = useRef(0);

  const reset = () => {
    run.current++;
    setStep("guide");
    setFailed(false);
    setQueries([]);
    setSearched(0);
    setFound([]);
    setNotFound([]);
    setAdding(false);
  };

  const handleOpenChange = (next: boolean) => {
    if (!next) reset();
    else {
      setKind(initialKind);
      setFormat(KIND_FORMATS[initialKind][0]);
    }
    onOpenChange(next);
  };

  const changeKind = (next: Kind) => {
    setKind(next);
    if (!KIND_FORMATS[next].includes(format)) setFormat(KIND_FORMATS[next][0]);
  };

  // Called from the tap itself: the camera only opens while the tap lasts.
  const shoot = (camera = true) => {
    const taking = takePhoto({ camera });
    reset();
    const id = run.current;
    void (async () => {
      const photo = await taking;
      if (photo === null || id !== run.current) return;
      setStep("reading");
      let texts: string[];
      try {
        texts = await shelfQueries(photo);
      } catch {
        texts = [];
      }
      if (id !== run.current) return;
      if (texts.length === 0) {
        setFailed(true);
        setStep("guide");
        return;
      }
      setQueries(texts);
      setStep("searching");
      const language = languageOf(i18n.language);
      const seen = new Set<string>();
      const pending = [...texts];
      const worker = async () => {
        for (let read = pending.shift(); read !== undefined; read = pending.shift()) {
          let candidate: Candidate | null = null;
          try {
            candidate = await shelfMatch(kind, read, language);
          } catch {
            // A search that fails is one work less, not a message each time.
          }
          if (id !== run.current) return;
          const key = candidate && `${candidate.source}:${candidate.externalId ?? candidate.title}`;
          if (candidate && key && !seen.has(key)) {
            seen.add(key);
            const match = candidate;
            setFound((all) => [
              ...all,
              {
                candidate: match,
                read,
                picked: match.existingEditionId === null && match.trashedEditionId === null,
              },
            ]);
          } else if (!candidate) {
            setNotFound((all) => [...all, read]);
          }
          setSearched((n) => n + 1);
        }
      };
      await Promise.all(Array.from({ length: AT_ONCE }, worker));
      if (id === run.current) setStep("review");
    })();
  };

  const toggle = (index: number) =>
    setFound((all) => all.map((f, i) => (i === index ? { ...f, picked: !f.picked } : f)));

  const addPicked = async () => {
    setAdding(true);
    const copy = {
      owner: owner.trim() || null,
      format,
      quantity: 1,
      location: location.trim() || null,
      acquiredOn: null,
    };
    const setStatus = (index: number, status: Found["status"]) =>
      setFound((all) => all.map((f, i) => (i === index ? { ...f, status } : f)));
    for (const [index, item] of found.entries()) {
      if (!item.picked || item.status === "added") continue;
      setStatus(index, "adding");
      try {
        // Picked while in the trash: it comes back as it was, with its own copies, instead of a second one (D105).
        const inTrash = item.candidate.trashedEditionId;
        if (inTrash) {
          await restoreFromTrash([{ kind: "edition", id: inTrash }]);
        } else {
          const book = newBookFromCandidate(item.candidate);
          await createBook({ ...book, copy: { ...book.copy, ...copy } });
        }
        setStatus(index, "added");
        onChanged();
      } catch {
        setStatus(index, "failed");
      }
    }
    setAdding(false);
  };

  const toAdd = found.filter((f) => f.picked && f.status !== "added").length;
  const added = found.filter((f) => f.status === "added").length;

  const guide = (
    <div className="flex flex-col gap-4 px-4 pt-2">
      {failed && (
        <p role="status" className="rounded-[14px] border p-3 text-sm">
          {t("shelf.nothingRead")}
        </p>
      )}
      <p className="text-[15px] text-pretty">{t("shelf.intro")}</p>
      <ul className="list-disc space-y-1 pl-5 text-sm text-pretty text-muted-foreground">
        <li>{t("shelf.tipOneShelf")}</li>
        <li>{t("shelf.tipLight")}</li>
        <li>{t("shelf.tipSpines")}</li>
      </ul>
      <Field id="shelf-kind" label={t("shelf.whatIsThere")}>
        <ChoiceSelect
          id="shelf-kind"
          value={kind}
          onChange={(v) => changeKind(v as Kind)}
          choices={kinds
            .filter((k) => k !== "other")
            .map((k) => ({ value: k, label: t(`typesPlural.${k}`) }))}
        />
      </Field>
      <p className="text-[13px] text-pretty text-muted-foreground">{t("shelf.privacy")}</p>
    </div>
  );

  const progress = (
    <div className="flex flex-col gap-2 px-4 pt-2">
      <p role="status" className="flex items-center gap-2 text-sm">
        <Loader2 className="size-4 animate-spin" />
        {step === "reading"
          ? t("shelf.reading")
          : t("shelf.searching", { done: searched, total: queries.length })}
      </p>
      {step === "searching" && (
        <div className="h-1.5 overflow-hidden rounded-full bg-muted">
          <div
            className="h-full bg-primary transition-[width]"
            style={{ width: `${(searched / Math.max(queries.length, 1)) * 100}%` }}
          />
        </div>
      )}
    </div>
  );

  const list = (
    <ul className="flex flex-col gap-1">
      {found.map((item, index) => {
        const { candidate } = item;
        const people =
          candidate.authors.length > 0
            ? candidate.authors
            : candidate.contributors.map((c) => c.name);
        const meta = [people.join(", "), candidate.publishedYear]
          .filter((part) => part !== null && part !== "")
          .join(" · ");
        return (
          <li key={`${candidate.source}-${candidate.externalId ?? index}`}>
            <label className="flex items-center gap-3 rounded-xl p-2">
              <input
                type="checkbox"
                className="size-5 shrink-0 accent-primary"
                checked={item.picked || item.status === "added"}
                disabled={adding || item.status === "added"}
                onChange={() => toggle(index)}
              />
              <Cover url={candidate.coverUrl} alt="" kind={candidate.kind} className="h-16" />
              <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                <span className="truncate text-[15px] font-semibold">{candidate.title}</span>
                {meta && (
                  <span className="truncate text-[13.5px] text-muted-foreground">{meta}</span>
                )}
                <span className="truncate text-xs text-muted-foreground">
                  {t("shelf.read", { text: item.read })}
                </span>
                {candidate.existingEditionId && (
                  <span className="text-[13px] font-medium text-primary">
                    {t("shelf.alreadyHere", { library })}
                  </span>
                )}
                {candidate.trashedEditionId && (
                  <span className="text-[13px] font-medium text-muted-foreground">
                    {t("shelf.inTrash")}
                  </span>
                )}
              </div>
              {item.status === "adding" && <Loader2 className="size-4 shrink-0 animate-spin" />}
              {item.status === "added" && (
                <CircleCheck
                  className="size-4 shrink-0 text-green-600"
                  aria-label={t("shelf.added")}
                />
              )}
              {item.status === "failed" && (
                <CircleAlert
                  className="size-4 shrink-0 text-destructive"
                  aria-label={t("shelf.failed")}
                />
              )}
            </label>
          </li>
        );
      })}
    </ul>
  );

  const copyFields = (
    <div className="grid gap-3 px-2 pt-2">
      <p className="text-sm font-medium">{t("shelf.copyDetails")}</p>
      <Field id="shelf-owner" label={t("form.fields.owner")}>
        <SuggestInput
          id="shelf-owner"
          suggestions={options.owners}
          value={owner}
          onValueChange={setOwner}
        />
      </Field>
      <Field id="shelf-format" label={t("form.fields.format")}>
        <ChoiceSelect
          id="shelf-format"
          value={format}
          onChange={(v) => setFormat(v as Format)}
          choices={KIND_FORMATS[kind].map((f) => ({ value: f, label: t(`formats.${f}`) }))}
        />
      </Field>
      <Field id="shelf-location" label={t("form.fields.location")}>
        <SuggestInput
          id="shelf-location"
          suggestions={options.locations}
          value={location}
          onValueChange={setLocation}
        />
      </Field>
    </div>
  );

  const unfound = notFound.length > 0 && (
    <details className="px-2 pt-3 text-sm">
      <summary className="cursor-pointer text-muted-foreground">
        {t("shelf.notFound", { count: notFound.length })}
      </summary>
      <p className="pt-2 text-[13px] text-pretty text-muted-foreground">
        {t("shelf.notFoundHint")}
      </p>
      <ul className="flex flex-col pt-1">
        {notFound.map((text) => (
          <li key={text}>
            <button
              type="button"
              className="flex w-full items-center gap-2 rounded-lg p-2 text-left"
              onClick={() => {
                handleOpenChange(false);
                onSearch(text);
              }}
            >
              <Search className="size-4 shrink-0 text-muted-foreground" />
              <span className="truncate">{text}</span>
            </button>
          </li>
        ))}
      </ul>
    </details>
  );

  const review = (
    <>
      <p role="status" className="px-2 pb-2 text-sm text-pretty">
        {added > 0
          ? t("shelf.addedSummary", { count: added })
          : t("shelf.summary", { count: found.length, read: queries.length })}
      </p>
      {list}
      {found.length > 0 && copyFields}
      {unfound}
      <ResultsCredits sources={found.map((f) => f.candidate.source)} />
    </>
  );

  const footer =
    step === "guide" ? (
      <div className="flex flex-col gap-2">
        <Button className="h-12 w-full gap-2 rounded-full" onClick={() => shoot()}>
          <Camera className="size-4" /> {t(failed ? "shelf.tryAgain" : "shelf.takePhoto")}
        </Button>
        <Button
          variant="outline"
          className="h-12 w-full gap-2 rounded-full"
          onClick={() => shoot(false)}
        >
          <ImageIcon className="size-4" /> {t("shelf.choosePhoto")}
        </Button>
      </div>
    ) : step === "review" ? (
      <div className="flex gap-2">
        <Button
          variant="outline"
          className="h-12 flex-1 gap-2 rounded-full"
          disabled={adding}
          onClick={() => shoot()}
        >
          <Camera className="size-4" /> {t("shelf.another")}
        </Button>
        {toAdd > 0 && (
          <Button
            className="h-12 flex-1 rounded-full"
            disabled={adding}
            onClick={() => void addPicked()}
          >
            {adding && <Loader2 className="animate-spin" />}
            {t("shelf.add", { count: toAdd })}
          </Button>
        )}
      </div>
    ) : null;

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <ScreenDialogContent>
        <div className="flex min-h-0 flex-1 flex-col">
          <header className="flex h-16 shrink-0 items-center gap-1 px-1">
            <Button
              type="button"
              variant="ghost"
              size="icon-lg"
              className="size-12 rounded-full"
              aria-label={t("common.close")}
              onClick={() => handleOpenChange(false)}
            >
              <X className="size-[22px]" />
            </Button>
            <DialogTitle className="flex-1 text-lg font-semibold">{t("shelf.title")}</DialogTitle>
          </header>
          <DialogDescription className="sr-only">{t("shelf.intro")}</DialogDescription>
          <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
            {step === "guide" && guide}
            {(step === "reading" || step === "searching") && progress}
            {(step === "searching" || step === "review") && (
              <div className="pt-3">{step === "review" ? review : list}</div>
            )}
          </div>
          {footer && (
            <div className="shrink-0 border-t px-4 pt-3 pb-[max(1.75rem,env(safe-area-inset-bottom))]">
              {footer}
            </div>
          )}
        </div>
      </ScreenDialogContent>
    </Dialog>
  );
}
